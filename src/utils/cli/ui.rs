use anyhow::Result;
use colored::Colorize;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{self, ClearType},
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};

use crate::cryptography::crypto;
use crate::cryptography::decrypt::{run_decrypt, run_encrypt};
use crate::cryptography::ease_trojan;
use crate::utils::pack_mode::resolve_pack_mode;
use crate::utils::filesystem::fs_ops;
use crate::{parse_hex_key, Cli};

use rust_i18n::t;

pub fn println_info(msg: &str) {
    println!("{}", msg.bright_blue().bold());
}

pub fn println_error(msg: &str) {
    eprintln!("{}", msg.bright_red().bold());
}

pub fn println_warn(msg: &str) {
    println!("{}", msg.bright_yellow());
}

pub fn create_progress_bar(len: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:30.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb.set_message(msg.to_string());
    pb
}

/// 创建 MultiProgress 管理器
pub fn create_multi_progress() -> MultiProgress {
    MultiProgress::new()
}

/// 在 MultiProgress 中添加进度条
pub fn add_progress_bar(multi: &MultiProgress, len: u64, msg: String) -> ProgressBar {
    let pb = multi.add(ProgressBar::new(len));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:30.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb.set_message(msg);
    pb
}

/// 交互式读取一行
pub async fn read_line(prompt: &str) -> Result<String> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    print!("{}", prompt);
    stdout().flush()?;
    let mut reader = BufReader::new(tokio::io::stdin());
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    Ok(line.trim().to_string())
}

/// 提示用户输入导出目录，默认值为原逻辑下的上级目录
pub async fn prompt_output_dir(default: &Path) -> Result<PathBuf> {
    println_info(&t!("prompt_output_dir", default = default.display().to_string()));
    println_info(&t!("prompt_output_dir_hint"));
    let input = read_line("> ").await?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(default.to_path_buf())
    } else {
        Ok(PathBuf::from(trimmed))
    }
}

/// 批量模式下询问基础输出目录，返回 `None` 表示使用默认规则
pub async fn prompt_output_base_dir() -> Result<Option<PathBuf>> {
    println_info(&t!("prompt_output_base_dir"));
    println_info(&t!("prompt_output_base_dir_hint"));
    let input = read_line("> ").await?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(trimmed)))
    }
}

/// TUI 多存档选择菜单
pub async fn select_saves(saves: Vec<(PathBuf, String)>) -> Result<Vec<PathBuf>> {
    if saves.is_empty() {
        return Ok(vec![]);
    }

    let mut stdout = stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let mut selected = vec![false; saves.len()];
    let mut cursor_pos = 0usize;

    loop {
        execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        println_info(&t!("batch_select_title"));
        for (i, (_, name)) in saves.iter().enumerate() {
            let prefix = if i == cursor_pos { ">" } else { " " };
            let check = if selected[i] { "[x]" } else { "[ ]" };
            let line = format!("{} {} {}", prefix, check, name);
            if selected[i] {
                println!("{}", line.bright_green());
            } else if i == cursor_pos {
                println!("{}", line.bright_white().on_blue());
            } else {
                println!("{}", line);
            }
        }
        let selected_count = selected.iter().filter(|&&x| x).count();
        println!(
            "\n{}",
            t!("batch_select_hint", selected = selected_count, total = saves.len()).dimmed()
        );

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Up => {
                        if cursor_pos > 0 {
                            cursor_pos -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if cursor_pos + 1 < saves.len() {
                            cursor_pos += 1;
                        }
                    }
                    KeyCode::Char(' ') => {
                        selected[cursor_pos] = !selected[cursor_pos];
                    }
                    KeyCode::Enter => break,
                    KeyCode::Esc => {
                        execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
                        terminal::disable_raw_mode()?;
                        anyhow::bail!("Selection cancelled");
                    }
                    _ => {}
                }
            }
        }
    }

    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;

    let mut result = Vec::new();
    for (i, sel) in selected.into_iter().enumerate() {
        if sel {
            result.push(saves[i].0.clone());
        }
    }
    Ok(result)
}

pub async fn process_single(save_path: &Path, cli: &Cli) -> Result<()> {
    println_info(&t!("test_integrity"));
    if let Err(e) = fs_ops::integrity_check(save_path).await {
        println_error(&format!("{}: {}", t!("integrity_fail"), e));
        return Err(anyhow::anyhow!("Integrity check failed"));
    }
    println_info(&t!("integrity_pass"));

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
        read_line(&t!("select_operation")).await?.parse::<usize>()?
    };

    let suffix = if op == 0 || op == 2 { "_Dec" } else { "_Enc" };
    let default_out_dir = {
        let parent = save_path.parent().unwrap_or(Path::new("../../.."));
        let name = save_path.file_name().unwrap_or_default();
        parent.join(format!("{}{}", name.to_string_lossy(), suffix))
    };
    let out_dir = if let Some(o) = &cli.output {
        PathBuf::from(o)
    } else {
        prompt_output_dir(&default_out_dir).await?
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

    // 创建进度条（如果有文件需要处理且不启用详情模式）
    let pb = if !cli.details && !file_list.is_empty() {
        Some(create_progress_bar(file_list.len() as u64, &progress_msg_raw))
    } else {
        None
    };

    match op {
        0 => {
            run_decrypt(save_path, &out_dir, &encrypted, &decrypted, None, pack_mode, pb.as_ref(), cli.details).await?;
        }
        1 => {
            run_encrypt(save_path, &out_dir, &decrypted, None, pack_mode, pb.as_ref(), cli.details).await?;
        }
        2 => {
            let key = if let Some(k) = &cli.key {
                parse_hex_key(k)?
            } else {
                // 使用原始文件做快速验证，避免重复代码
                let test_path = save_path.join("db").join(&encrypted[0]);
                let test_data = tokio::fs::read(&test_path).await?;
                match crypto::derive_key(&manifest_name, &current_data) {
                    Ok(k) if crypto::decrypt_data(&test_data, &k).is_ok() => k,
                    _ => {
                        let trojan = ease_trojan::EaseTrojan::new();
                        let derived = trojan.derive_key(save_path).await?;
                        println_info(&t!("key_success", key = hex::encode(derived)));
                        derived
                    }
                }
            };
            run_decrypt(save_path, &out_dir, &encrypted, &decrypted, Some(&key), pack_mode, pb.as_ref(), cli.details).await?;
        }
        3 => {
            let key = if let Some(k) = &cli.key {
                parse_hex_key(k)?
            } else {
                let msg = t!("invalid_key");
                anyhow::bail!(msg);
            };
            run_encrypt(save_path, &out_dir, &decrypted, Some(&key), pack_mode, pb.as_ref(), cli.details).await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

pub async fn process_batch(base_path: &Path, cli: &Cli) -> Result<()> {
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

    let selected = select_saves(saves_with_names).await?;
    if selected.is_empty() {
        println_warn(t!("no_saves_selected").as_ref());
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
    let cli_details = cli.details;
    let cli_key = cli.key.clone();

    // 询问基础输出目录（若未使用 -o）
    let base_output: Option<PathBuf> = if cli.output.is_some() {
        cli.output.clone().map(PathBuf::from)
    } else {
        prompt_output_base_dir().await?
    };

    let m = create_multi_progress();
    let mut tasks = Vec::new();

    for save in selected {
        let suffix = if op == 0 || op == 2 { "_Dec" } else { "_Enc" };
        let out_dir = if let Some(ref base) = base_output {
            // 指定了基础目录，存档同名子目录
            base.join(save.file_name().unwrap())
        } else {
            // 未指定，使用原默认规则：上级目录加后缀
            let parent = save.parent().unwrap_or(Path::new("../../.."));
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
                println_error(&format!("Scan error for {}: {}", save.display(), e));
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

        // 如果启用详情模式，则不创建进度条
        let pb = if cli_details {
            None
        } else {
            Some(add_progress_bar(
                &m,
                file_count,
                format!("{}", save.file_name().unwrap().to_string_lossy()),
            ))
        };

        // 如果启用详情模式，先打印存档名称作为分隔
        if cli_details {
            eprintln!("\n>>> {}: {} <<<", t!("process_save"), save.file_name().unwrap().to_string_lossy());
        }

        let cli_mode = op;
        let cli_key = cli_key.clone();
        let pack_mode = pack_mode;
        let save_path = save;
        let out_dir_path = out_dir;
        let encrypted_list = encrypted;
        let decrypted_list = decrypted;
        let manifest_name = manifest_name;
        let current_data = current_data;

        tasks.push(tokio::spawn(async move {
            let result = async {
                match cli_mode {
                    0 => {
                        run_decrypt(
                            &save_path,
                            &out_dir_path,
                            &encrypted_list,
                            &decrypted_list,
                            None,
                            pack_mode,
                            pb.as_ref(),
                            cli_details,
                        )
                            .await
                    }
                    1 => {
                        run_encrypt(
                            &save_path,
                            &out_dir_path,
                            &decrypted_list,
                            None,
                            pack_mode,
                            pb.as_ref(),
                            cli_details,
                        )
                            .await
                    }
                    2 => {
                        let key = if let Some(k) = &cli_key {
                            parse_hex_key(k)?
                        } else {
                            let test_path = save_path.join("db").join(&encrypted_list[0]);
                            let test_data = tokio::fs::read(&test_path).await?;
                            match crypto::derive_key(&manifest_name, &current_data) {
                                Ok(k) if crypto::decrypt_data(&test_data, &k).is_ok() => k,
                                _ => {
                                    let trojan = ease_trojan::EaseTrojan::new();
                                    let derived = trojan.derive_key(&save_path).await?;
                                    println_info(
                                        &t!("key_success", key = hex::encode(derived))
                                    );
                                    derived
                                }
                            }
                        };
                        run_decrypt(
                            &save_path,
                            &out_dir_path,
                            &encrypted_list,
                            &decrypted_list,
                            Some(&key),
                            pack_mode,
                            pb.as_ref(),
                            cli_details,
                        )
                            .await
                    }
                    3 => {
                        let key = if let Some(k) = &cli_key {
                            parse_hex_key(k)?
                        } else {
                            let msg = t!("invalid_key");
                            anyhow::bail!(msg);
                        };
                        run_encrypt(
                            &save_path,
                            &out_dir_path,
                            &decrypted_list,
                            Some(&key),
                            pack_mode,
                            pb.as_ref(),
                            cli_details,
                        )
                            .await
                    }
                    _ => unreachable!(),
                }
            };
            if let Err(e) = result.await {
                if let Some(pb) = pb.as_ref() {
                    pb.finish_with_message("Failed");
                }
                println_error(&format!("Error processing {}: {}", save_path.file_name().unwrap().to_string_lossy(), e));
            } else {
                if let Some(pb) = pb.as_ref() {
                    pb.finish_with_message("Done");
                }
            }
            Ok::<_, anyhow::Error>(())
        }));
    }

    for result in futures::future::join_all(tasks).await {
        if let Err(e) = result? {
            println_error(&format!("Task panicked: {}", e));
        }
    }

    if !cli_details {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    Ok(())
}