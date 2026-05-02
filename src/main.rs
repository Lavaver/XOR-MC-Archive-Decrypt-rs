#[macro_use]
extern crate rust_i18n;
i18n!("locales", fallback = "en");

mod utils;
pub mod cryptography;
pub mod network;

use anyhow::Result;
use clap::Parser;
use rust_i18n::t;
use std::path::PathBuf;

use crate::network::software_update;
use crate::utils::cli::parse::Cli;
use crate::utils::cli::ui::{self, process_batch, process_single};
use crate::utils::chunks::scan::{scan_chunks, infer_encrypted_chunks};
use crate::utils::chunks::overview::print_overview;
use crate::utils::filesystem::fs_ops;

#[tokio::main]
async fn main() -> Result<()> {
    auto_set_locale_culture();
    let cli = Cli::parse();

    ui::println_info(&t!("welcome"));

    // 如果不是专门执行更新，则仅检查更新并提示
    if !cli.sync {
        match software_update::check_for_updates().await {
            Ok(Some(v)) => {
                ui::println_info(&t!("update_available", version = v));
                #[cfg(target_os = "windows")]
                {
                    ui::println_info(&t!("use_sync_to_update"));
                }
            }
            Err(e) => {
                ui::println_warn(&format!("Update check failed: {}", e));
            }
            _ => {}
        }
    }

    // 如果指定了 --sync，则直接执行更新流程
    if cli.sync {
        ui::println_info(&t!("starting_update"));
        let pb = ui::create_progress_bar(0, &t!("downloading_update"));
        if let Err(e) = software_update::update(Some(pb)).await {
            ui::println_error(&format!("Update failed: {}", e));
        }
        return Ok(());
    }

    // 正常解密流程 - 获取路径
    let base_path = if let Some(p) = &cli.path {
        PathBuf::from(p)
    } else {
        let input = ui::read_line(&t!("tip_path")).await?;
        PathBuf::from(input)
    };

    if !base_path.exists() {
        ui::println_error(&t!("invalid_path"));
        anyhow::bail!(t!("invalid_path"));
    }

    // 区块扫描模式
    if cli.chunks {
        let result = scan_chunks(&base_path).await?;
        print_overview(&result);

        let encrypted_chunks = infer_encrypted_chunks(&result.plain);
        if !encrypted_chunks.is_empty() {
            println!("\n{}", t!("chunk_derived_encrypted_count"));
            for pos in &encrypted_chunks {
                let dim_name = match pos.dim {
                    0 => "Overworld",
                    1 => "Nether",
                    2 => "End",
                    _ => "Unknown",
                };
                println!("  ({}, {}) [{}]", pos.x, pos.z, dim_name);
            }
        }
        return Ok(());
    }

    let is_single = if cli.single {
        true
    } else if cli.batch {
        false
    } else if fs_ops::is_save_dir(&base_path).await {
        true
    } else {
        let sub = fs_ops::find_save_dirs(&base_path).await.unwrap_or_default();
        !sub.is_empty()
    };

    if is_single {
        process_single(&base_path, &cli).await?;
    } else {
        process_batch(&base_path, &cli).await?;
    }

    println!("{}", t!("exit"));
    Ok(())
}

fn parse_hex_key(input: &str) -> Result<[u8; 8]> {
    let bytes = hex::decode(input.trim_start_matches("0x"))?;
    if bytes.len() < 8 {
        let msg = t!("invalid_key");
        anyhow::bail!(msg);
    }
    let mut key = [0u8; 8];
    key.copy_from_slice(&bytes[bytes.len() - 8..]);
    Ok(key)
}

fn auto_set_locale_culture() {
    let system_locale = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
    let culture_code = if system_locale.starts_with("zh-CN") {
        "zh-CN"
    } else {
        "en"
    };
    rust_i18n::set_locale(culture_code);
}