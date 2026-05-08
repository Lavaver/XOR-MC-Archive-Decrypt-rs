use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use indicatif::ProgressBar;

use crate::utils::cli::ui;
use crate::utils::filesystem::aipe::CryptDewIoEngine;
use crate::utils::filesystem::{dir_setting::ensure_dir, fs_ops};
use crate::utils::filesystem::pack::{pack_mcworld_output, pack_tar_output};
use crate::utils::filesystem::pack_mode::PackMode;

pub async fn run_decrypt(
    src: &Path,
    out_dir: &Path,
    encrypted: &[String],
    _decrypted: &[String],
    key: Option<&[u8; 8]>,
    pack_mode: PackMode,
    pb: Option<&ProgressBar>,
    details: bool,
    archive_base: Option<&Path>,
) -> anyhow::Result<()> {
    if encrypted.is_empty() {
        if let Some(pb) = pb {
            pb.finish_with_message("No files");
        }
        return Ok(());
    }

    ensure_dir(out_dir).await?;
    fs_ops::copy_dir_all(src, out_dir).await?;

    let start = Instant::now();
    let _total_files = encrypted.len();
    let effective_key = key.unwrap_or(&[0u8; 8]).clone();

    // 设置进度回调
    let progress_callback: Option<Arc<dyn Fn(u64) + Send + Sync>> = if !details {
        if let Some(pb) = pb {
            let pb = pb.clone();
            Some(Arc::new(move |_processed| {
                pb.inc(1);
            }))
        } else {
            None
        }
    } else {
        None
    };

    // 使用异步 IO 原语引擎解密
    let engine = CryptDewIoEngine::new();
    engine
        .decrypt_files(src, out_dir, encrypted, effective_key, progress_callback)
        .await?;

    // 细节输出（仅在 details 模式下）
    if details {
        let total_bytes = 0u64; // 引擎已输出统计，此处不再重复
        let elapsed = start.elapsed().as_secs_f64();
        let speed_mb = if elapsed > 0.0 {
            total_bytes as f64 / elapsed / 1_000_000.0
        } else {
            0.0
        };
        eprintln!(
            "Decryption details: {:.2} MB/s (average)",
            speed_mb
        );
    }

    // 合理性检查
    if !details {
        ui::println_info(&t!("avail_test"));
        let ldb_ok = fs_ops::ldb_sanity_check(out_dir).await;
        let nbt_ok = fs_ops::nbt_sanity_check(out_dir).await;

        if ldb_ok && nbt_ok {
            ui::println_info(&t!("avail_pass"));
        } else {
            if !ldb_ok {
                ui::println_error(&t!("avail_fail"));
            }
            if !nbt_ok {
                ui::println_error(&t!("nbt_fail"));
            }
        }
    } else {
        eprintln!();
    }

    // 打包输出
    match pack_mode {
        PackMode::Copy => {
            ui::println_info(&t!("dec_success", path = out_dir.display().to_string()));
        }
        PackMode::Tar => {
            let tar_path = if let Some(base) = archive_base {
                base.with_extension("tar")
            } else {
                out_dir.with_extension("tar")
            };
            pack_tar_output(out_dir, &tar_path).await?;
            if let Err(e) = fs_ops::remove_dir_all(out_dir).await {
                ui::println_warn(&format!("{}: {}", t!("cleanup_temp_fail"), e));
            }
        }
        PackMode::McWorld => {
            let mcw_path = if let Some(base) = archive_base {
                base.with_extension("mcworld")
            } else {
                out_dir.with_extension("mcworld")
            };
            pack_mcworld_output(out_dir, &mcw_path).await?;
            if let Err(e) = fs_ops::remove_dir_all(out_dir).await {
                ui::println_warn(&format!("{}: {}", t!("cleanup_temp_fail"), e));
            }
        }
    }
    Ok(())
}

pub async fn run_encrypt(
    src: &Path,
    out_dir: &Path,
    decrypted: &[String],
    key: Option<&[u8; 8]>,
    pack_mode: PackMode,
    pb: Option<&ProgressBar>,
    details: bool,
    archive_base: Option<&Path>,
) -> anyhow::Result<()> {
    if decrypted.is_empty() {
        if let Some(pb) = pb {
            pb.finish_with_message("No files");
        }
        return Ok(());
    }

    ensure_dir(out_dir).await?;
    fs_ops::copy_dir_all(src, out_dir).await?;

    let start = Instant::now();
    let effective_key = key.unwrap_or(&[0u8; 8]).clone();

    let progress_callback: Option<Arc<dyn Fn(u64) + Send + Sync>> = if !details {
        if let Some(pb) = pb {
            let pb = pb.clone();
            Some(Arc::new(move |_processed| {
                pb.inc(1);
            }))
        } else {
            None
        }
    } else {
        None
    };

    let engine = CryptDewIoEngine::new();
    engine
        .encrypt_files(src, out_dir, decrypted, effective_key, progress_callback)
        .await?;

    if details {
        let elapsed = start.elapsed().as_secs_f64();
        eprintln!(
            "Encryption details: {:.2} s elapsed",
            elapsed
        );
    } else {
        eprintln!();
    }

    // 打包输出
    match pack_mode {
        PackMode::Copy => {
            ui::println_info(&t!("enc_success", path = out_dir.display().to_string()));
        }
        PackMode::Tar => {
            let tar_path = if let Some(base) = archive_base {
                base.with_extension("tar")
            } else {
                out_dir.with_extension("tar")
            };
            pack_tar_output(out_dir, &tar_path).await?;
            if let Err(e) = fs_ops::remove_dir_all(out_dir).await {
                ui::println_warn(&format!("{}: {}", t!("cleanup_temp_fail"), e));
            }
        }
        PackMode::McWorld => {
            let mcw_path = if let Some(base) = archive_base {
                base.with_extension("mcworld")
            } else {
                out_dir.with_extension("mcworld")
            };
            pack_mcworld_output(out_dir, &mcw_path).await?;
            if let Err(e) = fs_ops::remove_dir_all(out_dir).await {
                ui::println_warn(&format!("{}: {}", t!("cleanup_temp_fail"), e));
            }
        }
    }
    Ok(())
}