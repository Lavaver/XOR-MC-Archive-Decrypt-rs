#[macro_use]
extern crate rust_i18n;
i18n!("locales", fallback = "en");

mod utils;
pub mod cryptography;

// 3rd-Party Crate References
use anyhow::Result;
use clap::Parser;
use rust_i18n::t;
use std::path::PathBuf;

use crate::utils::cli::parse::Cli;
use crate::utils::cli::ui::{self, process_batch, process_single};
use utils::filesystem::fs_ops;

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

fn auto_set_locale_culture(){
    let system_locale = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
    let culture_code = if system_locale.starts_with("zh-CN"){
        "zh-CN"
    } else {
        "en"
    };
    rust_i18n::set_locale(culture_code);
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