use std::path::Path;

pub async fn run_sha256_hash_async(ctn: &str) -> Result<String, anyhow::Error> {
    let sha256 = chksum_sha2_256::async_chksum(ctn.as_bytes()).await;
    if sha256.is_err() {
        return Err(anyhow::anyhow!(
            "Failed to calculate sha256: {:?}",
            sha256.err()
        ));
    }
    let sha256 = sha256?;
    Ok(sha256.to_hex_lowercase())
}

pub async fn run_sha256_file_hash_async(path: &str) -> Result<String, anyhow::Error> {
    let sha256 = chksum_sha2_256::async_chksum(Path::new(path)).await;
    if sha256.is_err() {
        return Err(anyhow::anyhow!(
            "Failed to calculate sha256: {:?}",
            sha256.err()
        ));
    }
    let sha256 = sha256?;
    Ok(sha256.to_hex_lowercase())
}

pub fn run_md5_hash(ctn: &str) -> Result<String, anyhow::Error> {
    let md5 = chksum_md5::chksum(ctn.as_bytes());
    if md5.is_err() {
        return Err(anyhow::anyhow!("Failed to calculate md5: {:?}", md5.err()));
    }
    let md5 = md5?;
    Ok(md5.to_hex_lowercase())
}

pub async fn run_md5_hash_async(ctn: &str) -> Result<String, anyhow::Error> {
    let md5 = chksum_md5::async_chksum(ctn.as_bytes()).await;
    if md5.is_err() {
        return Err(anyhow::anyhow!("Failed to calculate md5: {:?}", md5.err()));
    }
    let md5 = md5?;
    Ok(md5.to_hex_lowercase())
}

pub async fn run_md5_file_hash_async(path: &str) -> Result<String, anyhow::Error> {
    let md5 = chksum_md5::async_chksum(Path::new(path)).await;
    if md5.is_err() {
        return Err(anyhow::anyhow!("Failed to calculate md5: {:?}", md5.err()));
    }
    let md5 = md5?;
    Ok(md5.to_hex_lowercase())
}
