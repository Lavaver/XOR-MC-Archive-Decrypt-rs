use crate::t;
use crate::utils::chunks::scan::{ChunkScanResult, infer_encrypted_chunks};

pub fn print_overview(result: &ChunkScanResult) {
    let plain_count = result.plain.len();
    let encrypted_chunks = infer_encrypted_chunks(&result.plain);
    let derived_enc_count = encrypted_chunks.len();
    let total = plain_count + derived_enc_count + result.encrypted_file_count;

    println!("=== {} ===", t!("chunk_overview_title"));
    println!("{}: {}", t!("chunk_plain_count"), plain_count);
    println!("{}: {}", t!("chunk_encrypted_file_count"), result.encrypted_file_count);
    println!("{}: {}", t!("chunk_derived_encrypted_count"), derived_enc_count);
    println!("{}: {}", t!("chunk_total_count"), total);
    if !encrypted_chunks.is_empty() {
        println!("\n{}:", t!("chunk_selectable_list"));
        for pos in &encrypted_chunks {
            let dim_name = match pos.dim {
                0 => "Overworld",
                1 => "Nether",
                2 => "End",
                _ => "Unknown",
            };
            println!("  ({}, {}) [{}]", pos.x, pos.z, dim_name);
        }
    }
}