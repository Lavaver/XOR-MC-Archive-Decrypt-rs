use anyhow::{Context, Result};
use std::path::Path;
use tokio::task::spawn_blocking;
use crate::utils::cli::ui;

/// 将目录打包为 tar 文件
async fn tar_directory(src: &Path, dst: &Path) -> Result<()> {
    let src = src.to_owned();
    let dst = dst.to_owned();

    spawn_blocking(move || -> Result<()> {
        let canonical_src = std::fs::canonicalize(&src)
            .context(t!("canonicalize_source_directory_fail"))?;

        if !canonical_src.is_dir() {
            anyhow::bail!(t!("not_a_directory_fail"));
        }

        let folder_name = canonical_src
            .file_name()
            .context(t!("determine_folder_name_fail"))?;

        let file = std::fs::File::create(&dst)
            .context(t!("create_destination_file_fail"))?;
        let mut builder = tar::Builder::new(file);
        builder
            .append_dir_all(folder_name, &canonical_src)
            .context(t!("append_directory_fail"))?;
        builder.finish()?;

        Ok(())
    })
        .await??;
    Ok(())
}

/// 将目录打包为 .mcworld 文件（ZIP 格式）
async fn zip_directory(src: &Path, dst: &Path) -> Result<()> {
    let src = src.to_owned();
    let dst = dst.to_owned();
    spawn_blocking(move || -> Result<()> {
        let file = std::fs::File::create(&dst)?;
        let mut writer = zip::ZipWriter::new(file);
        // 消除类型推断：显式指定 FileOptions<()>
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        for entry in walkdir::WalkDir::new(&src).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            let name = path.strip_prefix(&src)?;
            writer.start_file(name.to_string_lossy(), options)?;
            std::io::copy(&mut std::fs::File::open(path)?, &mut writer)?;
        }
        writer.finish()?;
        Ok(())
    })
        .await??;
    Ok(())
}

pub async fn pack_mcworld_output(dir: &Path) -> Result<()> {
    let mcw_path = dir.with_extension("mcworld");
    zip_directory(dir, &mcw_path).await?;
    ui::println_info(&t!("pack_mcworld", path = mcw_path.display().to_string()));
    Ok(())
}

pub async fn pack_tar_output(dir: &Path) -> Result<()> {
    let tar_path = dir.with_extension("tar");
    tar_directory(dir, &tar_path).await?;
    ui::println_info(&t!("pack_tar", path = tar_path.display().to_string()));
    Ok(())
}