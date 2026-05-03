use std::path::Path;
use std::time::Instant;
use indicatif::ProgressBar;
use tokio::fs;
use crate::cryptography::crypto;
use crate::utils::cli::ui;
use crate::utils::filesystem::{fs_ops, dir_setting::prepare_output_dir};
use crate::utils::pack::{pack_mcworld_output, pack_tar_output};
use crate::utils::pack_mode::PackMode;

pub async fn run_decrypt(
    src: &Path,
    out_dir: &Path,
    encrypted: &[String],
    _decrypted: &[String],
    key: Option<&[u8; 8]>,
    pack_mode: PackMode,
    pb: Option<&ProgressBar>,
    details: bool,
) -> anyhow::Result<()> {
    if encrypted.is_empty() {
        if let Some(pb) = pb {
            pb.finish_with_message("No files");
        }
        return Ok(());
    }

    prepare_output_dir(out_dir).await?;
    fs_ops::copy_dir_all(src, out_dir).await?;

    let start = Instant::now();
    let mut total_bytes = 0u64;
    let total_files = encrypted.len();

    for (idx, fname) in encrypted.iter().enumerate() {
        if !details {
            if let Some(pb) = pb {
                pb.set_message(format!("{}", fname));
            }
        }
        let file_path = out_dir.join("db").join(fname);
        let data = fs::read(&file_path).await?;
        let dec = crypto::decrypt_data(&data, key.unwrap_or(&[0u8; 8]))?;
        fs::write(&file_path, &dec).await?;

        if details {
            total_bytes += data.len() as u64;
            let elapsed = start.elapsed().as_secs_f64();
            let speed_mb = if elapsed > 0.0 {
                total_bytes as f64 / elapsed / 1_000_000.0
            } else {
                0.0
            };
            eprintln!(
                "[{}/{}] {} ({}: {:.2} MB/s, {}: {:.2} MB)",
                idx + 1,
                total_files,
                fname,
                t!("disk_speed"),
                speed_mb,
                t!("total_files"),
                total_bytes as f64 / 1_000_000.0
            );
        } else if let Some(pb) = pb {
            pb.inc(1);
        }
    }

    if !details {
        // 合理性检查仅在非详情模式下输出（避免混淆）
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
        eprintln!(); // 空行分隔详情和最终信息
    }

    match pack_mode {
        PackMode::Copy => {
            ui::println_info(&t!("dec_success", path = out_dir.display().to_string()));
        }
        PackMode::Tar => {
            pack_tar_output(out_dir).await?;
            // 清理临时目录
            if let Err(e) = fs_ops::remove_dir_all(out_dir).await {
                ui::println_warn(&format!("{}: {}", t!("cleanup_temp_fail"), e));
            }
        }
        PackMode::McWorld => {
            pack_mcworld_output(out_dir).await?;
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
) -> anyhow::Result<()> {
    if decrypted.is_empty() {
        if let Some(pb) = pb {
            pb.finish_with_message("No files");
        }
        return Ok(());
    }

    prepare_output_dir(out_dir).await?;
    fs_ops::copy_dir_all(src, out_dir).await?;

    let start = Instant::now();
    let mut total_bytes = 0u64;
    let total_files = decrypted.len();

    for (idx, fname) in decrypted.iter().enumerate() {
        if !details {
            if let Some(pb) = pb {
                pb.set_message(format!("{}", fname));
            }
        }
        let file_path = out_dir.join("db").join(fname);
        let data = fs::read(&file_path).await?;
        let enc = crypto::encrypt_data(&data, key.unwrap_or(&[0u8; 8]));
        fs::write(&file_path, &enc).await?;

        if details {
            total_bytes += data.len() as u64;
            let elapsed = start.elapsed().as_secs_f64();
            let speed_mb = if elapsed > 0.0 {
                total_bytes as f64 / elapsed / 1_000_000.0
            } else {
                0.0
            };
            eprintln!(
                "[{}/{}] {} ({}: {:.2} MB/s, {}: {:.2} MB)",
                idx + 1,
                total_files,
                fname,
                t!("disk_speed"),
                speed_mb,
                t!("total_files"),
                total_bytes as f64 / 1_000_000.0
            );
        } else if let Some(pb) = pb {
            pb.inc(1);
        }
    }

    if !details {
        // 加密后不需要做 sanity 检查（仅解密时验证）
        // 但为了保持一致性，此处留空
    } else {
        eprintln!();
    }

    match pack_mode {
        PackMode::Copy => {
            ui::println_info(&t!("enc_success", path = out_dir.display().to_string()));
        }
        PackMode::Tar => {
            pack_tar_output(out_dir).await?;
            if let Err(e) = fs_ops::remove_dir_all(out_dir).await {
                ui::println_warn(&format!("{}: {}", t!("cleanup_temp_fail"), e));
            }
        }
        PackMode::McWorld => {
            pack_mcworld_output(out_dir).await?;
            if let Err(e) = fs_ops::remove_dir_all(out_dir).await {
                ui::println_warn(&format!("{}: {}", t!("cleanup_temp_fail"), e));
            }
        }
    }
    Ok(())
}