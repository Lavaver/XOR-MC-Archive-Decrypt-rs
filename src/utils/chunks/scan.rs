use std::collections::HashSet;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
    pub dim: i32, // 0=主世界, 1=下界, 2=末地
}

pub struct ChunkScanResult {
    pub plain: HashSet<ChunkPos>,
    pub encrypted_file_count: usize,
}

fn decode_varint32(data: &[u8]) -> Option<(u32, usize)> {
    let mut value = 0u32;
    let mut shift = 0;
    for (i, &byte) in data.iter().enumerate() {
        value |= ((byte & 0x7F) as u32) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            return Some((value, i + 1));
        }
        if i >= 4 {
            break;
        }
    }
    None
}

fn is_chunk_key(key: &[u8]) -> Option<ChunkPos> {
    if key.len() < 9 {
        return None;
    }
    let x = i32::from_le_bytes(key[0..4].try_into().ok()?);
    let z = i32::from_le_bytes(key[4..8].try_into().ok()?);
    let dim = if key.len() >= 13 {
        let d = i32::from_le_bytes(key[8..12].try_into().ok()?);
        if d < -1 || d > 2 {
            return None;
        }
        d
    } else {
        0
    };
    let tag_offset = if key.len() >= 13 { 12 } else { 8 };
    let tag = key[tag_offset];
    if !(0x2D..=0x41).contains(&tag) && tag != 0x76 && tag != 0x77 {
        return None;
    }
    Some(ChunkPos { x, z, dim })
}

fn parse_unencrypted_ldb(data: &[u8], chunks: &mut HashSet<ChunkPos>) {
    let mut offset = 0;
    while offset + 4 < data.len() {
        // 尝试读取一个记录
        let (shared_len, bytes) = match decode_varint32(&data[offset..]) {
            Some(v) => v,
            None => break,
        };
        offset += bytes;

        let (non_shared_len, bytes) = match decode_varint32(&data[offset..]) {
            Some(v) => v,
            None => break,
        };
        offset += bytes;

        let (value_len, bytes) = match decode_varint32(&data[offset..]) {
            Some(v) => v,
            None => break,
        };
        offset += bytes;

        // 如果 shared_len 为 0 且 non_shared_len > 0，则是完整键
        if shared_len == 0 && non_shared_len > 0 && value_len > 0 {
            let key_end = offset + non_shared_len as usize;
            let val_end = key_end + value_len as usize;
            if val_end > data.len() {
                break;
            }
            let key = &data[offset..key_end];
            // 检查是否为区块键
            if let Some(pos) = is_chunk_key(key) {
                chunks.insert(pos);
            }
            offset = val_end;
        } else {
            // 不能解析的记录，跳过1字节继续尝试
            offset += 1;
        }

        // 防止无限循环
        if offset >= data.len() {
            break;
        }
    }
}

pub async fn scan_chunks(save_path: &Path) -> anyhow::Result<ChunkScanResult> {
    let db_path = save_path.join("db");
    // 利用已有函数扫描 db 目录
    let (encrypted_files, decrypted_files, _, _) = crate::utils::filesystem::fs_ops::scan_db(&db_path).await?;

    let mut plain_positions = HashSet::new();

    for fname in decrypted_files.iter().filter(|f| f.ends_with(".ldb")) {
        let file_path = db_path.join(fname);
        let data = fs::read(&file_path).await?;
        parse_unencrypted_ldb(&data, &mut plain_positions);
    }

    Ok(ChunkScanResult {
        plain: plain_positions,
        encrypted_file_count: encrypted_files.len(),
    })
}

pub fn infer_encrypted_chunks(plain: &HashSet<ChunkPos>) -> Vec<ChunkPos> {
    let mut candidates = HashSet::new();
    for pos in plain {
        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let candidate = ChunkPos {
                    x: pos.x + dx,
                    z: pos.z + dz,
                    dim: pos.dim,
                };
                if !plain.contains(&candidate) {
                    candidates.insert(candidate);
                }
            }
        }
    }
    let mut result: Vec<ChunkPos> = candidates.into_iter().collect();
    result.sort_by_key(|p| (p.dim, p.x, p.z));
    result
}