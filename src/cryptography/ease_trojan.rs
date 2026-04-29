use anyhow::{Context, Result};
use shen_nbt5::{NbtValue, nbt_version};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

use crate::cryptography::crypto;
use crate::utils::filesystem::fs_ops;
use crate::t;

#[derive(Debug, Clone)]
struct Anchor {
    offset: isize,
    plaintext: &'static [u8],
}

pub struct EaseTrojan {
    anchors: Vec<Anchor>,
}

impl EaseTrojan {
    pub fn new() -> Self {
        let anchors = vec![
            Anchor {
                offset: -8,
                plaintext: &[0x57, 0xFB, 0x80, 0x8B, 0x24, 0x74, 0x47, 0xDB],
            },
            Anchor {
                offset: 8,
                plaintext: &[0x0A],
            },
            Anchor {
                offset: 0,
                plaintext: &[0x00],
            },
        ];
        EaseTrojan { anchors }
    }

    pub async fn derive_key(&self, save_path: &Path) -> Result<[u8; 8]> {
        let db_path = save_path.join("db");
        let (encrypted, _, _, _) = fs_ops::scan_db(&db_path).await?;
        let sample_file = encrypted
            .iter()
            .find(|f| f.ends_with(".ldb"))
            .context(t!("no_encrypted"))?;
        let sample_path = db_path.join(sample_file);
        let ciphertext = fs::read(&sample_path).await?;

        // 收集密钥片段
        let mut fragments: Vec<Vec<u8>> = Vec::new();
        for anchor in &self.anchors {
            if let Some(matched) = self.extract_bytes(&ciphertext, anchor) {
                let key_part: Vec<u8> = anchor
                    .plaintext
                    .iter()
                    .zip(matched)
                    .map(|(p, c)| p ^ c)
                    .collect();
                fragments.push(key_part);
            }
        }
        if fragments.is_empty() {
            anyhow::bail!(t!("no_anchor_matched"));
        }

        // 生成所有可能的候选密钥（每个字节位置的投票结果可能产生多个候选）
        let candidates = Self::generate_candidates(&fragments)?;

        // 依次验证
        for key in candidates {
            if self.verify_key(&sample_path, &key).await.is_ok() {
                return Ok(key);
            }
        }

        anyhow::bail!(t!("all_keys_failed"))
    }

    fn extract_bytes<'a>(&self, data: &'a [u8], anchor: &Anchor) -> Option<&'a [u8]> {
        let len = anchor.plaintext.len();
        if data.len() < len {
            return None;
        }
        let start = if anchor.offset < 0 {
            let s = data.len() as isize + anchor.offset;
            if s < 0 {
                return None;
            }
            s as usize
        } else {
            anchor.offset as usize
        };
        if start + len > data.len() {
            return None;
        }
        Some(&data[start..start + len])
    }

    fn generate_candidates(fragments: &[Vec<u8>]) -> Result<Vec<[u8; 8]>> {
        let mut byte_candidates: Vec<Vec<u8>> = vec![Vec::new(); 8];

        for i in 0..8 {
            let mut counts = HashMap::new();
            for frag in fragments {
                if let Some(&val) = frag.get(i) {
                    *counts.entry(val).or_insert(0) += 1;
                }
            }
            if counts.is_empty() {
                anyhow::bail!(t!("no_votes_for_byte", byte = i));
            }
            // 收集所有出现过且具有最高票数的字节（或多个平票）
            let max_count = *counts.values().max().unwrap();
            let top_bytes: Vec<u8> = counts
                .into_iter()
                .filter(|&(_, cnt)| cnt == max_count)
                .map(|(b, _)| b)
                .collect();
            byte_candidates[i] = top_bytes;
        }

        // 计算笛卡尔积的总组合数，限制以防爆炸
        let total: usize = byte_candidates.iter().map(|v| v.len()).product();
        if total > 1_000_000 {
            anyhow::bail!(t!("too_many_key_candidates", count = total));
        }

        // 生成所有组合（8 层嵌套，支持每字节的候选数不同）
        let mut keys = Vec::with_capacity(total);
        let mut current = [0u8; 8];

        // 递归生成
        fn generate(
            idx: usize,
            byte_candidates: &[Vec<u8>],
            current: &mut [u8; 8],
            result: &mut Vec<[u8; 8]>,
        ) {
            if idx == 8 {
                result.push(*current);
                return;
            }
            for &b in &byte_candidates[idx] {
                current[idx] = b;
                generate(idx + 1, byte_candidates, current, result);
            }
        }
        generate(0, &byte_candidates, &mut current, &mut keys);
        Ok(keys)
    }

    async fn verify_key(&self, file_path: &Path, key: &[u8; 8]) -> Result<()> {
        let data = fs::read(file_path).await?;
        let dec = crypto::decrypt_data(&data, key)
            .context(t!("decrypt_test_block_fail"))?;

        // level.dat 需要执行 NBT 解析
        if file_path.file_name().map_or(false, |n| n == "level.dat") {
            if dec.len() <= 8 {
                anyhow::bail!(t!("level_dat_too_short"));
            }
            let nbt_data = &dec[8..];
            NbtValue::from_binary::<nbt_version::BedrockDisk>(&mut nbt_data.to_vec())
                .context(t!("nbt_verification_fail"))?;
        } else {
            // 其他文件（.ldb 等）检查尾部魔数
            if dec.len() < 8 {
                anyhow::bail!(t!("ldb_too_short"));
            }
            let tail = &dec[dec.len() - 8..];
            let expected: [u8; 8] = [0x57, 0xFB, 0x80, 0x8B, 0x24, 0x74, 0x47, 0xDB];
            if tail != expected {
                anyhow::bail!(t!("tail_magic_mismatch"));
            }
        }
        Ok(())
    }
}