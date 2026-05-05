use std::path::Path;
use tokio::fs;

/// 确保目录存在（不询问覆盖，仅创建）
pub async fn ensure_dir(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).await?;
    }
    Ok(())
}