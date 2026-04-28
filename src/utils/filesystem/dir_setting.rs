use std::path::Path;
use tokio::fs;
use crate::utils::cli::ui;
use crate::utils::filesystem::fs_ops;

pub async fn prepare_output_dir(path: &Path) -> anyhow::Result<()> {
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