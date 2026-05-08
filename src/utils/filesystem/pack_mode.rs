use crate::utils::cli::ui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackMode {
    /// 原样复制（不打包）
    Copy,
    /// 打包为 tar 归档
    Tar,
    /// 打包为 .mcworld（ZIP 格式）
    McWorld,
}

impl PackMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "0" | "copy" | "folder" | "dir" => Some(PackMode::Copy),
            "1" | "tar" => Some(PackMode::Tar),
            "2" | "mcworld" | "zip" | "mcw" => Some(PackMode::McWorld),
            _ => None,
        }
    }
}

pub async fn choose_pack_mode() -> anyhow::Result<PackMode> {
    ui::println_info(&t!("select_pack_mode"));
    loop {
        let input = ui::read_line("> ").await?;
        if let Some(mode) = PackMode::from_str(&input) {
            return Ok(mode);
        }
        ui::println_error(&t!("invalid_pack_mode"));
    }
}

pub async fn resolve_pack_mode(cli_pack_mode: &Option<String>) -> anyhow::Result<PackMode> {
    if let Some(pm) = cli_pack_mode {
        Ok(PackMode::from_str(pm).unwrap_or_else(|| {
            ui::println_error(&t!("invalid_pack_mode"));
            PackMode::Copy
        }))
    } else {
        choose_pack_mode().await
    }
}