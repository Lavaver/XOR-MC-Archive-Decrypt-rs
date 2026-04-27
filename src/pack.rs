use anyhow::Result;
use std::path::Path;
use tokio::task::spawn_blocking;

/// 将目录打包为 tar 文件
pub async fn tar_directory(src: &Path, dst: &Path) -> Result<()> {
    let src = src.to_owned();
    let dst = dst.to_owned();
    spawn_blocking(move || -> Result<()> {
        let file = std::fs::File::create(&dst)?;
        let mut builder = tar::Builder::new(file);
        builder.append_dir_all(".", &src)?;
        builder.finish()?;
        Ok(())
    })
        .await??;
    Ok(())
}

/// 将目录打包为 .mcworld 文件（ZIP 格式）
pub async fn zip_directory(src: &Path, dst: &Path) -> Result<()> {
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