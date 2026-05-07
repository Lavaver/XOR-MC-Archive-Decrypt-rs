use anyhow::{Context, Result};
use indicatif::ProgressBar;
use rust_i18n::t;
use serde_json::Value;
use std::io::Read;

use crate::utils::cli::ui;

/// 同步获取最新 release 信息（在 spawn_blocking 中调用）
fn fetch_latest_release_info_sync(owner: &str, repo: &str) -> Result<(String, String)> {
    let url = format!("https://api.github.com/repos/{}/{}/releases/latest", owner, repo);
    let resp = ureq::get(&url)
        .header("User-Agent", "update-manager")
        .call()?;

    let body: Value = serde_json::from_reader(resp.into_body().as_reader())?;

    let tag = body["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("{}", t!("no_release_tag_fail")))?
        .to_owned();

    let assets = body["assets"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("{}", t!("assets_not_an_array_fail")))?;

    let asset = assets
        .iter()
        .find(|a| a["name"].as_str().map_or(false, |n| n.ends_with(".exe")))
        .ok_or_else(|| anyhow::anyhow!("{}", t!("no_asset_fail")))?;

    let download_url = asset["browser_download_url"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("{}", t!("no_download_link_fail")))?
        .to_owned();

    Ok((tag, download_url))
}

/// 检查更新（异步接口）
pub async fn check_for_updates() -> Result<Option<String>> {
    let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .expect(t!("is_not_semver").as_ref());   // 编译时自动获取，无需手动写死

    let (tag, _) = tokio::task::spawn_blocking(move || {
        fetch_latest_release_info_sync("Lavaver", "Crypt-Dew-World")
    }).await??;

    let remote_version_str = tag.strip_prefix('v').unwrap_or(&tag);
    let remote_version = semver::Version::parse(remote_version_str)
        .context(t!("is_not_semver"))?;

    if remote_version > current_version {
        Ok(Some(remote_version.to_string()))
    } else {
        Ok(None)
    }
}

/// 执行自动更新（仅在 Windows 下替换自身）
pub async fn update(pb: Option<ProgressBar>) -> Result<()> {
    let new_version = match check_for_updates().await? {
        Some(v) => v,
        None => {
            ui::println_info(&t!("already_latest_version"));
            return Ok(());
        }
    };

    #[cfg(target_os = "windows")]
    {
        ui::println_info(&t!("downloading_version", version = &new_version));

        let (_tag, exe_url) = tokio::task::spawn_blocking(move || {
            fetch_latest_release_info_sync("Lavaver", "Crypt-Dew-World")
        })
            .await??;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("crypt-dew-world.exe");

        // 发起下载请求，获取响应
        let resp = ureq::get(&exe_url).call()?;

        // 先提取 Content-Length（如果可用）
        let total_size = resp
            .headers()
            .get("Content-Length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        let pb = pb.unwrap_or_else(|| {
            ui::create_progress_bar(total_size.unwrap_or(0), &t!("downloading_update"))
        });
        if let Some(size) = total_size {
            pb.set_length(size);
        }

        let mut body = resp.into_body();
        let mut reader = body.as_reader();
        let mut dest = tokio::fs::File::create(&temp_path).await?;
        let mut downloaded: u64 = 0;
        let mut buf = [0u8; 8192];

        loop {
            let bytes_read = reader.read(&mut buf)?;
            if bytes_read == 0 {
                break;
            }
            tokio::io::copy(&mut buf[..bytes_read].as_ref(), &mut dest).await?;
            downloaded += bytes_read as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message(t!("downloaded_and_use_patch"));

        let temp_path_clone = temp_path.clone();
        tokio::task::spawn_blocking(move || self_replace::self_replace(&temp_path_clone))
            .await??;

        let _ = tokio::fs::remove_file(&temp_path).await;

        ui::println_info(&t!("please_restart_application"));

        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        ui::println_warn(&t!("update_not_supported"));
        Ok(())
    }
}