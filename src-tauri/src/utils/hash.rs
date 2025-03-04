use std::path::Path;

pub async fn run_hash(path: &str) -> Result<String, String> {
    let sha256 = chksum_sha2_256::async_chksum(Path::new(path)).await;
    if sha256.is_err() {
        return Err(format!("Failed to calculate sha256: {:?}", sha256.err()));
    }
    let sha256 = sha256.unwrap();
    return Ok(sha256.to_hex_lowercase());
}
