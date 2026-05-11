use anyhow::{bail, Result};
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::fs;
use shen_nbt5::{NbtValue, nbt_version};

use crate::cryptography::crypto::is_encrypted;
use crate::utils::cli::ui;
use rust_i18n::t;

/// Check whether the directory contains the `level.dat` file and the `db` subdirectory
pub async fn is_save_dir(path: &Path) -> bool {
    let level_dat = path.join("level.dat");
    let db = path.join("db");
    level_dat.is_file() && db.is_dir()
}

/// Search for all valid archive directories under `base_path`
pub async fn find_save_dirs(base_path: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    let mut entries = fs::read_dir(base_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() && is_save_dir(&path).await {
            dirs.push(path);
        }
    }
    Ok(dirs)
}

/// If possible, retrieve the name of the save file for that world
pub async fn read_level_name(save_path: &Path) -> String {
    let name_file = save_path.join("levelname.txt");
    match fs::read_to_string(&name_file).await {
        Ok(name) => name.trim().to_string(),
        Err(_) => String::new(),
    }
}

/// Integrity check `level.dat`, the `db` directory, `db/MANIFEST-XXXXXX`, and `db/CURRENT`
pub async fn integrity_check(save_path: &Path) -> Result<()> {
    // 1. 检查 level.dat
    let level_dat_path = save_path.join("level.dat");
    if !level_dat_path.is_file() {
        let msg = t!("need_but_missing", name = "level.dat");
        ui::println_error(&msg);
        bail!(msg);
    }
    // 2. 检查 db 目录
    let db_path = save_path.join("db");
    if !db_path.is_dir() {
        let msg = t!("need_but_missing", name = "db/");
        ui::println_error(&msg);
        bail!(msg);
    }
    // 3. 检查 db 内的 MANIFEST-* 和 CURRENT
    let mut has_manifest = false;
    let mut has_current = false;
    let manifest_re = Regex::new(r"^MANIFEST-\d{6}$")?;

    let mut entries = fs::read_dir(&db_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let fname = entry.file_name();
        let fname = fname.to_string_lossy();
        if manifest_re.is_match(&fname) {
            has_manifest = true;
        } else if fname == "CURRENT" {
            has_current = true;
        }
        if has_manifest && has_current {
            break;
        }
    }

    if !has_manifest {
        let msg = t!("need_but_missing", name = "MANIFEST-*");
        ui::println_error(&msg);
        bail!(msg);
    }
    if !has_current {
        let msg = t!("need_but_missing", name = "CURRENT");
        ui::println_error(&msg);
        bail!(msg);
    }
    Ok(())
}

/// Scan Database Directory
pub async fn scan_db(db_path: &Path) -> Result<(Vec<String>, Vec<String>, String, Vec<u8>)> {
    let mut encrypted = Vec::new();
    let mut decrypted = Vec::new();
    let mut manifest_name = String::new();
    let mut current_data = Vec::new();

    let manifest_re = Regex::new(r"^MANIFEST-\d{6}$")?;
    let ldb_re = Regex::new(r"^\d{6}\.ldb$")?;

    let mut entries = fs::read_dir(db_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let fname = entry.file_name().to_string_lossy().to_string();
        let file_path = entry.path();
        if !manifest_re.is_match(&fname) && fname != "CURRENT" && !ldb_re.is_match(&fname) {
            continue;
        }
        if file_path.is_file() {
            let data = fs::read(&file_path).await?;
            if is_encrypted(&data) {
                encrypted.push(fname.clone());
            } else {
                decrypted.push(fname.clone());
            }
            if manifest_re.is_match(&fname) {
                manifest_name = fname.clone();
            }
            if fname == "CURRENT" {
                current_data = data;
            }
        }
    }
    Ok((encrypted, decrypted, manifest_name, current_data))
}

/// ldb Magical Number Check
pub async fn ldb_sanity_check(save_path: &Path) -> bool {
    let db_path = save_path.join("db");
    let expect: u64 = 0x57FB808B247547DB;
    if let Ok(mut entries) = fs::read_dir(&db_path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "ldb") {
                if let Ok(data) = fs::read(&path).await {
                    if data.len() < 8 {
                        return false;
                    }
                    let last8 = &data[data.len() - 8..];
                    let val = match u64::from_be_bytes(last8.try_into().expect("Failed to convert last 8 bytes to u64")) {
                        v => v,
                    };
                    if val != expect {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
    }
    true
}

/// Recursively copy the directory, automatically skipping the `netease_config` directory
pub async fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).await?;
    let mut entries = fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let src_entry = entry.path();
        let fname = entry.file_name();
        // 忽略 netease_config 文件夹，防止解密后导入失败
        if fname.to_string_lossy() == "netease_config" && src_entry.is_dir() {
            continue;
        }
        let dst_entry = dst.join(fname);
        if src_entry.is_dir() {
            Box::pin(copy_dir_all(&src_entry, &dst_entry)).await?;
        } else {
            fs::copy(&src_entry, &dst_entry).await?;
        }
    }
    Ok(())
}

/// Delete Directory
pub async fn remove_dir_all(path: &Path) -> Result<()> {
    fs::remove_dir_all(path).await?;
    Ok(())
}

/// Verify that the archived NBT data matches expectations
pub async fn nbt_sanity_check(save_path: &Path) -> bool {
    // 第一步：检查 level.dat
    let level_dat_path = save_path.join("level.dat");
    if let Ok(data) = fs::read(&level_dat_path).await {
        if data.len() <= 8 {
            return false;
        }
        let nbt_data = data[8..].to_vec();
        if NbtValue::from_binary::<nbt_version::BedrockDisk>(&mut nbt_data.clone()).is_err() {
            return false;
        }
    } else {
        return false;
    }

    // 第二步：抽样检查 db 目录下的 NBT 数据
    let db_path = save_path.join("db");
    if let Ok(mut entries) = fs::read_dir(&db_path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                if fname.starts_with("BlockEntity") || fname.starts_with("Entity") {
                    if let Ok(mut data) = fs::read(&path).await {
                        if NbtValue::from_binary::<nbt_version::BedrockDisk>(&mut data).is_err() {
                            return false;
                        }
                    }
                }
            }
        }
    }
    true
}

/// 判断目录是否为空（不存在也视为空）
pub async fn is_dir_empty(path: &Path) -> Result<bool> {
    if !path.is_dir() {
        return Ok(true);
    }
    let mut entries = fs::read_dir(path).await?;
    Ok(entries.next_entry().await?.is_none())
}