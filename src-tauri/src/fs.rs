use fmmap::tokio::AsyncMmapFileExt;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{local::mmap, utils::hash::run_sha256_file_hash_async, REQUEST_CLIENT};

pub async fn create_http_stream(
    url: &str,
    offset: usize,
    size: usize,
) -> Result<
    (
        Box<dyn tokio::io::AsyncRead + Unpin + std::marker::Send>,
        u64,
    ),
    String,
> {
    let mut res = REQUEST_CLIENT.get(url);
    let has_range = offset > 0 || size > 0;
    if has_range {
        res = res.header("Range", format!("bytes={}-{}", offset, offset + size - 1));
        println!("Range: bytes={}-{}", offset, offset + size - 1);
    }
    let res = res.send().await;
    if res.is_err() {
        return Err(format!("Failed to send http request: {:?}", res.err()));
    }
    let res = res.unwrap();
    let code = res.status();
    if (!has_range && code != 200) || (has_range && code != 206) {
        return Err(format!("Failed to download: URL {} returned {}", url, code));
    }
    let content_length = res.content_length().unwrap_or(0);
    let stream = futures::TryStreamExt::map_err(res.bytes_stream(), std::io::Error::other);
    let reader = tokio_util::io::StreamReader::new(stream);
    Ok((Box::new(reader), content_length))
}

pub async fn create_local_stream(
    offset: usize,
    size: usize,
) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + std::marker::Send>, String> {
    let mmap_file = mmap().await;
    let reader = mmap_file
        .range_reader(offset, size)
        .map_err(|e| format!("Failed to mmap: {:?}", e))?;
    Ok(Box::new(reader))
}

pub async fn prepare_target(target: &str) -> Result<Option<PathBuf>, String> {
    let target = Path::new(&target);
    let exe_path = std::env::current_exe();
    let mut override_path = None;
    if let Ok(exe_path) = exe_path {
        // check if target is the same as exe path
        if exe_path == target && exe_path.exists() {
            // if same, rename the exe to exe.old
            let old_exe = exe_path.with_extension("instbak");
            // delete old_exe if exists
            let _ = tokio::fs::remove_file(&old_exe).await;
            // rename current exe to old_exe
            let res = tokio::fs::rename(&exe_path, &old_exe).await;
            if res.is_err() {
                return Err(format!("Failed to rename current exe: {:?}", res.err()));
            }
            override_path = Some(old_exe);
        }
    }
    // ensure dir
    let parent = target.parent();
    if parent.is_none() {
        return Err("Failed to get parent dir".to_string());
    }
    let parent = parent.unwrap();
    let res = tokio::fs::create_dir_all(parent).await;
    if res.is_err() {
        return Err(format!("Failed to create parent dir: {:?}", res.err()));
    }
    Ok(override_path)
}

pub async fn create_target_file(target: &str) -> Result<impl AsyncWrite, String> {
    let target_file = tokio::fs::File::create(target).await;
    if target_file.is_err() {
        return Err(format!(
            "Failed to create target file: {:?}",
            target_file.err()
        ));
    }
    let target_file = target_file.unwrap();
    let target_file = tokio::io::BufWriter::new(target_file);
    Ok(target_file)
}

pub async fn progressed_copy(
    mut source: impl AsyncRead + std::marker::Unpin,
    mut target: impl AsyncWrite + std::marker::Unpin,
    on_progress: impl Fn(usize),
) -> Result<usize, String> {
    let mut downloaded = 0;
    let mut boxed = Box::new([0u8; 256 * 1024]);
    let buffer = &mut *boxed;
    let mut now = std::time::Instant::now();
    loop {
        let read: Result<usize, std::io::Error> = source.read(buffer).await;
        if read.is_err() {
            return Err(format!("Failed to read from decoder: {:?}", read.err()));
        }
        let read = read.unwrap();
        if read == 0 {
            break;
        }
        downloaded += read;
        // emit only every 16 ms
        if now.elapsed().as_millis() >= 20 {
            now = std::time::Instant::now();
            on_progress(downloaded);
        }
        let write = target.write_all(&buffer[..read]).await;
        if write.is_err() {
            return Err(format!("Failed to write to target file: {:?}", write.err()));
        }
    }
    // flush the buffer
    let res = target.flush().await;
    if res.is_err() {
        return Err(format!("Failed to flush target file: {:?}", res.err()));
    }
    // emit the final progress
    on_progress(downloaded);
    Ok(downloaded)
}

pub async fn verify_hash(target: &str, sha256: String) -> Result<(), String> {
    let hash = run_sha256_file_hash_async(target).await?;
    if hash != sha256 {
        return Err(format!(
            "File {} hash mismatch: expected {}, got {}",
            target, sha256, hash
        ));
    }
    Ok(())
}
