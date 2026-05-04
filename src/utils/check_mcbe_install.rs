pub fn detect_minecraft_be() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // UWP 版检测
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-AppxPackage -Name 'Microsoft.MinecraftUWP'",
            ])
            .output();
        if let Ok(out) = output {
            if !out.stdout.is_empty() {
                return true;
            }
        }

        // GDK 版检测（1.21+）
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = std::path::PathBuf::from(appdata).join("Minecraft Bedrock");
            if p.is_dir() {
                return true;
            }
        }

        // 两种检测都未命中 → 未安装
        false
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// 获取 Minecraft Bedrock (1.21+) 的 minecraftWorlds 根目录，失败返回 None
pub fn minecraft_worlds_root() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").ok()?;
        let bedrock_dir = std::path::PathBuf::from(&appdata).join("Minecraft Bedrock");
        if !bedrock_dir.is_dir() {
            return None;
        }

        let users_dir = bedrock_dir.join("Users");
        let dir_iter = std::fs::read_dir(&users_dir).ok()?;

        for entry in dir_iter.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                    // 用户 ID 通常为 19 位数字
                    if fname.len() >= 19 && fname.chars().all(|c| c.is_ascii_digit()) {
                        let worlds = path
                            .join("games")
                            .join("com.mojang")
                            .join("minecraftWorlds");
                        if worlds.is_dir() {
                            return Some(worlds);
                        }
                    }
                }
            }
        }

        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}