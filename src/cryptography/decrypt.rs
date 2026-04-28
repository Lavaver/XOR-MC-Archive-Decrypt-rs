use std::path::Path;
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
) -> anyhow::Result<()> {
    if encrypted.is_empty() {
        if let Some(pb) = pb {
            pb.finish_with_message("No files");
        }
        return Ok(());
    }

    prepare_output_dir(out_dir).await?;
    fs_ops::copy_dir_all(src, out_dir).await?;

    for fname in encrypted {
        if let Some(pb) = pb {
            pb.set_message(format!("{}", fname));
        }
        let file_path = out_dir.join("db").join(fname);
        let data = fs::read(&file_path).await?;
        let dec = crypto::decrypt_data(&data, key.unwrap_or(&[0u8; 8]))?;
        fs::write(&file_path, &dec).await?;
        if let Some(pb) = pb {
            pb.inc(1);
        }
    }

    // 合理性检查
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

    match pack_mode {
        PackMode::Copy => {
            ui::println_info(&t!("dec_success", path = out_dir.display().to_string()));
        }
        PackMode::Tar => {
            pack_tar_output(out_dir).await?;
        }
        PackMode::McWorld => {
            pack_mcworld_output(out_dir).await?;
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
) -> anyhow::Result<()> {
    if decrypted.is_empty() {
        if let Some(pb) = pb {
            pb.finish_with_message("No files");
        }
        return Ok(());
    }

    prepare_output_dir(out_dir).await?;
    fs_ops::copy_dir_all(src, out_dir).await?;

    for fname in decrypted {
        if let Some(pb) = pb {
            pb.set_message(format!("{}", fname));
        }
        let file_path = out_dir.join("db").join(fname);
        let data = fs::read(&file_path).await?;
        let enc = crypto::encrypt_data(&data, key.unwrap_or(&[0u8; 8]));
        fs::write(&file_path, &enc).await?;
        if let Some(pb) = pb {
            pb.inc(1);
        }
    }

    match pack_mode {
        PackMode::Copy => {
            ui::println_info(&t!("enc_success", path = out_dir.display().to_string()));
        }
        PackMode::Tar => {
            pack_tar_output(out_dir).await?;
        }
        PackMode::McWorld => {
            pack_mcworld_output(out_dir).await?;
        }
    }
    Ok(())
}