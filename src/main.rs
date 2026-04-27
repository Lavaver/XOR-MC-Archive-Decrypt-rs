#[macro_use]
extern crate rust_i18n;

i18n!("locales", fallback = "en");

mod crypto;
mod fs_ops;
mod pack;
mod ui;

use anyhow::Result;
use clap::Parser;
use indicatif::ProgressBar;
use std::path::{Path, PathBuf};
use tokio::fs;

use rust_i18n::t;

/// 打包模式
#[derive(Debug, Clone, Copy, PartialEq)]
enum PackMode {
    /// 原样复制（不打包）
    Copy,
    /// 打包为 tar 归档
    Tar,
    /// 打包为 .mcworld（ZIP 格式）
    McWorld,
}

impl PackMode {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "0" | "copy" | "folder" | "dir" => Some(PackMode::Copy),
            "1" | "tar" => Some(PackMode::Tar),
            "2" | "mcworld" | "zip" | "mcw" => Some(PackMode::McWorld),
            _ => None,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "mcsaveencrypt-rs")]
struct Cli {
    /// Path to save folder or directory containing multiple saves
    path: Option<String>,

    /// Force single save mode
    #[arg(short = 's', long)]
    single: bool,

    /// Force batch mode (scan subdirectories)
    #[arg(short = 'b', long)]
    batch: bool,

    /// Operation: decrypt(0) or encrypt(1) or specific (2,3)
    #[arg(short = 'm', long)]
    mode: Option<String>,

    /// Custom hex key (64-bit)
    #[arg(short = 'k', long)]
    key: Option<String>,

    /// Output directory
    #[arg(short = 'o', long)]
    output: Option<String>,

    /// Pack mode: copy, tar, mcworld (or 0,1,2)
    #[arg(short = 'P', long)]
    pack_mode: Option<String>,
}

fn init_logging() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap_or_else(|_| {
        log4rs::init_config(
            log4rs::config::Config::builder()
                .appender(
                    log4rs::config::Appender::builder().build(
                        "stdout",
                        Box::new(log4rs::append::console::ConsoleAppender::builder().build()),
                    ),
                )
                .build(
                    log4rs::config::Root::builder()
                        .appender("stdout")
                        .build(log::LevelFilter::Info),
                )
                .unwrap(),
        )
            .expect("Logger init failed");
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    auto_set_locale_culture();
    init_logging();
    let cli = Cli::parse();

    ui::println_info(&t!("welcome"));

    let base_path = if let Some(p) = &cli.path {
        PathBuf::from(p)
    } else {
        let input = ui::read_line(&t!("tip_path")).await?;
        PathBuf::from(input)
    };

    if !base_path.exists() {
        ui::println_error(&t!("invalid_path"));
        let msg = t!("invalid_path");
        anyhow::bail!(msg);
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

async fn process_single(save_path: &Path, cli: &Cli) -> Result<()> {
    ui::println_info(&t!("test_integrity"));
    if let Err(e) = fs_ops::integrity_check(save_path).await {
        ui::println_error(&format!("{}: {}", t!("integrity_fail"), e));
        return Err(anyhow::anyhow!("Integrity check failed"));
    }
    ui::println_info(&t!("integrity_pass"));

    let op = if let Some(m) = &cli.mode {
        match m.to_lowercase().as_str() {
            "decrypt" | "dec" | "0" => 0,
            "encrypt" | "enc" | "1" => 1,
            "2" => 2,
            "3" => 3,
            _ => {
                let msg = t!("invalid_op");
                anyhow::bail!(msg);
            }
        }
    } else {
        ui::read_line(&t!("select_operation")).await?.parse::<usize>()?
    };

    let suffix = if op == 0 || op == 2 { "_Dec" } else { "_Enc" };
    let out_dir = if let Some(o) = &cli.output {
        PathBuf::from(o)
    } else {
        let parent = save_path.parent().unwrap_or(Path::new("."));
        let name = save_path.file_name().unwrap_or_default();
        parent.join(format!("{}{}", name.to_string_lossy(), suffix))
    };

    let db_path = save_path.join("db");
    let (encrypted, decrypted, manifest_name, current_data) = fs_ops::scan_db(&db_path).await?;

    // 决定打包模式
    let pack_mode = resolve_pack_mode(&cli.pack_mode).await?;

    // 决定要处理的文件列表和进度条消息
    let (file_list, progress_msg_raw): (&[String], String) = match op {
        0 => (&encrypted, t!("decrypting").to_string()),
        1 => (&decrypted, t!("encrypting").to_string()),
        2 => (&encrypted, t!("decrypting").to_string()),
        3 => (&decrypted, t!("encrypting").to_string()),
        _ => unreachable!(),
    };

    // 创建进度条（如果有文件需要处理）
    let pb = if !file_list.is_empty() {
        Some(ui::create_progress_bar(file_list.len() as u64, &progress_msg_raw))
    } else {
        None
    };

    match op {
        0 => {
            run_decrypt(save_path, &out_dir, &encrypted, &decrypted, None, pack_mode, pb.as_ref()).await?;
        }
        1 => {
            run_encrypt(save_path, &out_dir, &decrypted, None, pack_mode, pb.as_ref()).await?;
        }
        2 => {
            let key = if let Some(k) = &cli.key {
                parse_hex_key(k)?
            } else {
                let k = crypto::derive_key(&manifest_name, &current_data)
                    .map_err(|e| anyhow::anyhow!("{}: {}", t!("key_fail"), e))?;
                ui::println_info(&t!("key_success", key = hex::encode(k)));
                k
            };
            run_decrypt(save_path, &out_dir, &encrypted, &decrypted, Some(&key), pack_mode, pb.as_ref()).await?;
        }
        3 => {
            let key = if let Some(k) = &cli.key {
                parse_hex_key(k)?
            } else {
                let msg = t!("invalid_key");
                anyhow::bail!(msg);
            };
            run_encrypt(save_path, &out_dir, &decrypted, Some(&key), pack_mode, pb.as_ref()).await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

async fn process_batch(base_path: &Path, cli: &Cli) -> Result<()> {
    let sub_dirs = fs_ops::find_save_dirs(base_path).await?;
    if sub_dirs.is_empty() {
        let msg = format!("No valid save directories found in {}", base_path.display());
        anyhow::bail!(msg);
    }

    let mut saves_with_names = Vec::new();
    for dir in sub_dirs {
        let name = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
        let level_name = fs_ops::read_level_name(&dir).await;
        let display = if level_name.is_empty() {
            name.clone()
        } else {
            format!("{} [{}]", name, level_name)
        };
        saves_with_names.push((dir, display));
    }

    let selected = ui::select_saves(saves_with_names).await?;
    if selected.is_empty() {
        ui::println_warn("No saves selected.");
        return Ok(());
    }

    let mode = cli.mode.as_deref().unwrap_or("0");
    let op = match mode {
        "0" | "decrypt" | "dec" => 0,
        "1" | "encrypt" | "enc" => 1,
        "2" => 2,
        "3" => 3,
        _ => {
            let msg = t!("invalid_op");
            anyhow::bail!(msg);
        }
    };

    // 确定统一的打包模式
    let pack_mode = resolve_pack_mode(&cli.pack_mode).await?;

    let m = ui::create_multi_progress();
    let mut tasks = Vec::new();

    for save in selected {
        let suffix = if op == 0 || op == 2 { "_Dec" } else { "_Enc" };
        let out_dir = if let Some(o) = &cli.output {
            PathBuf::from(o).join(save.file_name().unwrap())
        } else {
            let parent = save.parent().unwrap_or(Path::new("."));
            parent.join(format!(
                "{}{}",
                save.file_name().unwrap().to_string_lossy(),
                suffix
            ))
        };

        let db_path = save.join("db");
        let scan_result = fs_ops::scan_db(&db_path).await;
        let (encrypted, decrypted, manifest_name, current_data) = match scan_result {
            Ok(res) => res,
            Err(e) => {
                ui::println_error(&format!("Scan error for {}: {}", save.display(), e));
                continue;
            }
        };

        let file_count = match op {
            0 => encrypted.len() as u64,
            1 => decrypted.len() as u64,
            2 | 3 => {
                if op == 2 { encrypted.len() as u64 } else { decrypted.len() as u64 }
            }
            _ => unreachable!(),
        };

        if file_count == 0 {
            continue;
        }

        let pb = ui::add_progress_bar(
            &m,
            file_count,
            format!("{}", save.file_name().unwrap().to_string_lossy()),
        );

        let cli_mode = op;
        let cli_key = cli.key.clone();

        tasks.push(tokio::spawn(async move {
            let result = async {
                match cli_mode {
                    0 => run_decrypt(&save, &out_dir, &encrypted, &decrypted, None, pack_mode, Some(&pb)).await,
                    1 => run_encrypt(&save, &out_dir, &decrypted, None, pack_mode, Some(&pb)).await,
                    2 => {
                        let key = if let Some(k) = &cli_key {
                            parse_hex_key(k)?
                        } else {
                            let k = crypto::derive_key(&manifest_name, &current_data)
                                .map_err(|e| anyhow::anyhow!("{}: {}", t!("key_fail"), e))?;
                            k
                        };
                        run_decrypt(&save, &out_dir, &encrypted, &decrypted, Some(&key), pack_mode, Some(&pb)).await
                    }
                    3 => {
                        let key = if let Some(k) = &cli_key {
                            parse_hex_key(k)?
                        } else {
                            let msg = t!("invalid_key");
                            anyhow::bail!(msg);
                        };
                        run_encrypt(&save, &out_dir, &decrypted, Some(&key), pack_mode, Some(&pb)).await
                    }
                    _ => unreachable!(),
                }
            };
            if let Err(e) = result.await {
                pb.finish_with_message("Failed");
                ui::println_error(&format!("Error processing {}: {}", save.file_name().unwrap().to_string_lossy(), e));
            } else {
                pb.finish_with_message("Done");
            }
            Ok::<_, anyhow::Error>(())
        }));
    }

    for result in futures::future::join_all(tasks).await {
        if let Err(e) = result? {
            ui::println_error(&format!("Task panicked: {}", e));
        }
    }

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    Ok(())
}

async fn run_decrypt(
    src: &Path,
    out_dir: &Path,
    encrypted: &[String],
    _decrypted: &[String],
    key: Option<&[u8; 8]>,
    pack_mode: PackMode,
    pb: Option<&ProgressBar>,
) -> Result<()> {
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

async fn run_encrypt(
    src: &Path,
    out_dir: &Path,
    decrypted: &[String],
    key: Option<&[u8; 8]>,
    pack_mode: PackMode,
    pb: Option<&ProgressBar>,
) -> Result<()> {
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

async fn choose_pack_mode() -> Result<PackMode> {
    ui::println_info(&t!("select_pack_mode"));
    loop {
        let input = ui::read_line("> ").await?;
        if let Some(mode) = PackMode::from_str(&input) {
            return Ok(mode);
        }
        ui::println_error(&t!("invalid_pack_mode"));
    }
}

async fn prepare_output_dir(path: &Path) -> Result<()> {
    if path.exists() {
        let answer = ui::read_line(&t!("folder_exists_overwrite")).await?;
        if answer.to_lowercase() != "y" {
            anyhow::bail!("Aborted by user");
        }
        fs_ops::remove_dir_all(path).await?;
    }
    fs::create_dir_all(path).await?;
    Ok(())
}

async fn pack_tar_output(dir: &Path) -> Result<()> {
    let tar_path = dir.with_extension("tar");
    pack::tar_directory(dir, &tar_path).await?;
    ui::println_info(&t!("pack_tar", path = tar_path.display().to_string()));
    Ok(())
}

async fn pack_mcworld_output(dir: &Path) -> Result<()> {
    let mcw_path = dir.with_extension("mcworld");
    pack::zip_directory(dir, &mcw_path).await?;
    ui::println_info(&t!("pack_mcworld", path = mcw_path.display().to_string()));
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

async fn resolve_pack_mode(cli_pack_mode: &Option<String>) -> Result<PackMode> {
    if let Some(pm) = cli_pack_mode {
        Ok(PackMode::from_str(pm).unwrap_or_else(|| {
            ui::println_error(&t!("invalid_pack_mode"));
            PackMode::Copy
        }))
    } else {
        choose_pack_mode().await
    }
}

fn auto_set_locale_culture(){
    let system_locale = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
    let culture_code = if system_locale.starts_with("zh-CN"){
        "zh-CN"
    } else {
        "en"
    };
    rust_i18n::set_locale(culture_code);
}