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
use std::path::PathBuf;

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