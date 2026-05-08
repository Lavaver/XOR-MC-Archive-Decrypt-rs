use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Semaphore;

use crate::utils::cli::ui;

/// 块级所有权的数据单元
#[derive(Debug)]
pub struct Block {
    pub source_file: PathBuf,
    pub index: u64,
    pub data: Vec<u8>,
}

impl Block {
    pub fn new(source_file: PathBuf, index: u64, data: Vec<u8>) -> Self {
        Self {
            source_file,
            index,
            data,
        }
    }
}

/// I/O 引擎配置
#[derive(Debug, Clone)]
pub struct IoEngineConfig {
    pub block_size: usize,
    pub _max_concurrent_io: usize,
    pub read_concurrency: usize,
    pub write_concurrency: usize,
}

impl Default for IoEngineConfig {
    fn default() -> Self {
        Self {
            block_size: 256 * 1024,
            _max_concurrent_io: 32,
            read_concurrency: 4,
            write_concurrency: 4,
        }
    }
}

/// 三折叠 I/O 引擎
pub struct IoEngine {
    config: IoEngineConfig,
    read_semaphore: Arc<Semaphore>,
    write_semaphore: Arc<Semaphore>,
}

impl IoEngine {
    pub fn new(config: IoEngineConfig) -> Self {
        Self {
            read_semaphore: Arc::new(Semaphore::new(config.read_concurrency)),
            write_semaphore: Arc::new(Semaphore::new(config.write_concurrency)),
            config,
        }
    }

    pub fn default() -> Self {
        Self::new(IoEngineConfig::default())
    }

    /// 读取文件并分块
    pub async fn read_file_blocks(
        &self,
        file_path: &Path,
        file_name: &str,
    ) -> Result<Vec<Block>> {
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_size = metadata.len();
        if file_size == 0 {
            return Ok(Vec::new());
        }

        let block_count =
            (file_size + self.config.block_size as u64 - 1) / self.config.block_size as u64;
        let mut blocks = Vec::with_capacity(block_count as usize);
        let mut read_tasks = Vec::new();
        let file_path = Arc::new(file_path.to_path_buf());
        let file_name = file_name.to_string();

        for block_idx in 0..block_count {
            let file_path = file_path.clone();
            let file_name = file_name.clone();
            let block_size = self.config.block_size;
            let offset = block_idx * block_size as u64;
            let semaphore = self.read_semaphore.clone();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.map_err(|e| {
                    anyhow::anyhow!("Failed to acquire read semaphore: {}", e)
                })?;

                let mut file = tokio::fs::File::open(file_path.as_path()).await?;
                file.seek(std::io::SeekFrom::Start(offset)).await?;

                let remaining = file_size.saturating_sub(offset);
                let actual_block_size = std::cmp::min(block_size as u64, remaining) as usize;
                let mut buffer = vec![0u8; actual_block_size];
                file.read_exact(&mut buffer).await?;

                Ok::<_, anyhow::Error>(Block::new(
                    PathBuf::from(file_name),
                    block_idx,
                    buffer,
                ))
            });

            read_tasks.push(task);
        }

        for task in read_tasks {
            blocks.push(task.await??);
        }
        blocks.sort_by_key(|b| b.index);
        Ok(blocks)
    }

    /// 并发处理块（处理器使用 Arc 共享所有权）
    pub async fn process_blocks<F, Fut>(
        &self,
        blocks: Vec<Block>,
        processor: Arc<F>,
    ) -> Result<Vec<Block>>
    where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<u8>>> + Send + 'static,
    {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        let total_blocks = blocks.len();
        let mut processed_tasks = Vec::with_capacity(total_blocks);

        for block in blocks {
            let processor = Arc::clone(&processor);
            let task = tokio::spawn(async move {
                let processed_data = processor(block.data).await?;
                Ok::<_, anyhow::Error>(Block::new(
                    block.source_file,
                    block.index,
                    processed_data,
                ))
            });
            processed_tasks.push(task);
        }

        let mut result_blocks = Vec::with_capacity(total_blocks);
        for task in processed_tasks {
            result_blocks.push(task.await??);
        }
        result_blocks.sort_by_key(|b| b.index);
        Ok(result_blocks)
    }

    /// 写入块到目标目录
    pub async fn write_blocks(
        &self,
        blocks: Vec<Block>,
        base_dir: &Path,
        sub_dir: &str,
    ) -> Result<()> {
        if blocks.is_empty() {
            return Ok(());
        }

        let mut file_groups: HashMap<PathBuf, Vec<Block>> = HashMap::new();
        for block in blocks {
            file_groups
                .entry(block.source_file.clone())
                .or_default()
                .push(block);
        }

        let mut write_tasks = Vec::new();
        let semaphore = self.write_semaphore.clone();
        let base_dir = base_dir.to_path_buf();
        let sub_dir = sub_dir.to_string();

        for (file_name, mut file_blocks) in file_groups {
            file_blocks.sort_by_key(|b| b.index);
            let semaphore = semaphore.clone();
            let base_dir = base_dir.clone();
            let sub_dir = sub_dir.clone();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.map_err(|e| {
                    anyhow::anyhow!("Failed to acquire write semaphore: {}", e)
                })?;

                let file_path = base_dir.join(&sub_dir).join(&file_name);
                if let Some(parent) = file_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                let mut file = tokio::fs::File::create(&file_path).await?;
                for block in file_blocks {
                    file.write_all(&block.data).await?;
                }
                file.flush().await?;
                Ok::<_, anyhow::Error>(())
            });

            write_tasks.push(task);
        }

        for task in write_tasks {
            task.await??;
        }
        Ok(())
    }

    /// 完整的流水线处理
    pub async fn process_files<F, Fut>(
        &self,
        src_dir: &Path,
        out_dir: &Path,
        file_names: &[String],
        sub_dir: &str,
        block_processor: F,
        progress_callback: Option<Arc<dyn Fn(u64) + Send + Sync>>,
    ) -> Result<()>
    where
        F: Fn(Vec<u8>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<u8>>> + Send + 'static,
    {
        if file_names.is_empty() {
            return Ok(());
        }

        let processor = Arc::new(block_processor);
        let start = Instant::now();
        let total_files = file_names.len();
        let mut total_bytes_processed: u64 = 0;

        for (file_idx, file_name) in file_names.iter().enumerate() {
            let file_path = src_dir.join(sub_dir).join(file_name);
            if !file_path.exists() {
                ui::println_warn(&format!("File not found, skipping: {}", file_name));
                continue;
            }

            let blocks = self.read_file_blocks(&file_path, file_name).await?;
            if blocks.is_empty() {
                continue;
            }

            let file_bytes: u64 = blocks.iter().map(|b| b.data.len() as u64).sum();
            total_bytes_processed += file_bytes;

            let processed_blocks = self.process_blocks(blocks, Arc::clone(&processor)).await?;
            self.write_blocks(processed_blocks, out_dir, sub_dir).await?;

            if let Some(ref cb) = progress_callback {
                cb(file_idx as u64 + 1);
            }
        }

        let total_elapsed = start.elapsed();
        let total_speed_mb = if total_elapsed.as_secs_f64() > 0.0 {
            total_bytes_processed as f64 / total_elapsed.as_secs_f64() / 1_000_000.0
        } else {
            0.0
        };

        ui::println_info(&format!(
            "Blocks I/O complete | {} files | {} MB total | {:.2} MB/s average",
            total_files,
            total_bytes_processed / 1_000_000,
            total_speed_mb
        ));
        Ok(())
    }
}

/// 为 Crypt-Dew-World 定制的 I/O 引擎封装
pub struct CryptDewIoEngine {
    engine: IoEngine,
}

impl CryptDewIoEngine {
    pub fn new() -> Self {
        Self {
            engine: IoEngine::default(),
        }
    }

    pub async fn decrypt_files(
        &self,
        src_dir: &Path,
        out_dir: &Path,
        encrypted_files: &[String],
        key: [u8; 8],
        progress_callback: Option<Arc<dyn Fn(u64) + Send + Sync>>,
    ) -> Result<()> {
        use crate::cryptography::crypto;

        let key = Arc::new(key);
        self.engine
            .process_files(
                src_dir,
                out_dir,
                encrypted_files,
                "db",
                move |data| {
                    let key = *key;
                    async move {
                        crypto::decrypt_data(&data, &key).context("Block decryption failed")
                    }
                },
                progress_callback,
            )
            .await
    }

    pub async fn encrypt_files(
        &self,
        src_dir: &Path,
        out_dir: &Path,
        decrypted_files: &[String],
        key: [u8; 8],
        progress_callback: Option<Arc<dyn Fn(u64) + Send + Sync>>,
    ) -> Result<()> {
        use crate::cryptography::crypto;

        let key = Arc::new(key);
        self.engine
            .process_files(
                src_dir,
                out_dir,
                decrypted_files,
                "db",
                move |data| {
                    let key = *key;
                    async move { Ok(crypto::encrypt_data(&data, &key)) }
                },
                progress_callback,
            )
            .await
    }
}