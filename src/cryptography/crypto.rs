use anyhow::{bail, Result};

const DEFAULT_KEY: &[u8; 8] = b"88329851";

/// 检查数据是否带有加密魔数
pub fn is_encrypted(buf: &[u8]) -> bool {
    if buf.len() < 4 {
        return false;
    }
    let mag = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
    mag == 0x801D3001 || mag == 0x901D3001
}

/// 加密数据，返回带魔数的完整加密字节
pub fn encrypt_data(data: &[u8], key: &[u8; 8]) -> Vec<u8> {
    let effective_key: &[u8] = if key == &[0u8; 8] { &DEFAULT_KEY[..] } else { key };
    let mut out = vec![0x80, 0x1D, 0x30, 0x01];
    out.extend(
        data.iter()
            .enumerate()
            .map(|(i, b)| b ^ effective_key[i % effective_key.len()]),
    );
    out
}

/// 解密数据（输入必须包含魔数前缀）
pub fn decrypt_data(data: &[u8], key: &[u8; 8]) -> Result<Vec<u8>> {
    if !is_encrypted(data) {
        bail!("Data is not encrypted (magic mismatch)");
    }
    let effective_key: &[u8] = if key == &[0u8; 8] { &DEFAULT_KEY[..] } else { key };
    let payload = &data[4..];
    let dec: Vec<u8> = payload
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ effective_key[i % effective_key.len()])
        .collect();
    Ok(dec)
}

/// 从 MANIFEST 文件名和 CURRENT 内容推导密钥
pub fn derive_key(manifest_name: &str, current_data: &[u8]) -> Result<[u8; 8]> {
    if current_data.len() < 20 {
        bail!("CURRENT file too short");
    }
    let buf1 = format!("{}\n", manifest_name).into_bytes();
    let buf2 = &current_data[4..]; // 跳过前 4 字节

    if buf1.len() != buf2.len() {
        bail!("Length mismatch between MANIFEST name and CURRENT content tail");
    }

    let xored: Vec<u8> = buf1
        .iter()
        .zip(buf2.iter())
        .map(|(a, b)| a ^ b)
        .collect();
    if xored.len() < 16 {
        bail!("XOR result too short");
    }
    let first8 = &xored[0..8];
    let second8 = &xored[8..16];
    if first8 != second8 {
        bail!("Key halves do not match");
    }
    let mut key = [0u8; 8];
    key.copy_from_slice(first8);
    Ok(key)
}