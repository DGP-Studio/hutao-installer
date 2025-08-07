use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use crate::{REQUEST_CLIENT, capture_and_return_err};

fn get_optimal_download_threads() -> usize {
    let cpu_threads = num_cpus::get();
    cpu_threads.clamp(2, 8)
}

pub async fn create_http_stream(
    url: &str,
    offset: usize,
    size: usize,
) -> Result<Box<dyn AsyncRead + Unpin + Send>, anyhow::Error> {
    let mut res = REQUEST_CLIENT.get(url);
    let has_range = offset > 0 || size > 0;
    if has_range {
        res = res.header("Range", format!("bytes={}-{}", offset, offset + size - 1));
        println!("Range: bytes={}-{}", offset, offset + size - 1);
    }
    let res = res.send().await;
    if res.is_err() {
        return Err(anyhow::anyhow!(
            "Failed to send http request: {:?}",
            res.err()
        ));
    }
    let res = res?;
    let code = res.status();
    if (!has_range && code != 200) || (has_range && code != 206) {
        return Err(anyhow::anyhow!(
            "Failed to download: URL {} returned {}",
            url,
            code
        ));
    }
    let stream = futures::TryStreamExt::map_err(res.bytes_stream(), std::io::Error::other);
    let reader = tokio_util::io::StreamReader::new(stream);
    Ok(Box::new(reader))
}

pub async fn create_target_file(target: &str) -> Result<impl AsyncWrite, anyhow::Error> {
    let target_file = tokio::fs::File::create(target).await;
    if target_file.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to create target file: {:?}",
            target_file.err()
        ));
    }
    let target_file = target_file?;
    let target_file = tokio::io::BufWriter::new(target_file);
    Ok(target_file)
}

pub async fn progressed_copy(
    mut source: impl AsyncRead + Unpin,
    mut target: impl AsyncWrite + Unpin,
    on_progress: impl Fn(usize),
) -> Result<usize, anyhow::Error> {
    let mut downloaded = 0;
    let mut boxed = Box::new([0u8; 256 * 1024]);
    let buffer = &mut *boxed;
    let mut now = std::time::Instant::now();
    loop {
        let read: Result<usize, std::io::Error> = source.read(buffer).await;
        if read.is_err() {
            return Err(anyhow::anyhow!(
                "Failed to read from decoder: {:?}",
                read.err()
            ));
        }
        let read = read?;
        if read == 0 {
            break;
        }
        downloaded += read;
        if now.elapsed().as_millis() >= 20 {
            now = std::time::Instant::now();
            on_progress(downloaded);
        }
        let write = target.write_all(&buffer[..read]).await;
        if write.is_err() {
            return Err(anyhow::anyhow!(
                "Failed to write to target file: {:?}",
                write.err()
            ));
        }
    }
    let res = target.flush().await;
    if res.is_err() {
        return Err(anyhow::anyhow!(
            "Failed to flush target file: {:?}",
            res.err()
        ));
    }
    on_progress(downloaded);
    Ok(downloaded)
}

pub async fn check_range_support(url: &str) -> Result<bool, anyhow::Error> {
    let res = REQUEST_CLIENT.head(url).send().await?;
    Ok(res
        .headers()
        .get("accept-ranges")
        .is_some_and(|v| v == "bytes"))
}

pub async fn get_content_length(url: &str) -> Result<u64, anyhow::Error> {
    let res = REQUEST_CLIENT.head(url).send().await?;
    Ok(res.content_length().unwrap_or(0))
}

async fn get_final_url(url: &str) -> Result<String, anyhow::Error> {
    let res = REQUEST_CLIENT.head(url).send().await?;
    let final_url = res.url().to_string();
    Ok(final_url)
}

async fn multi_threaded_download_impl(
    url: &str,
    target: &str,
    chunk_count: usize,
    on_progress: impl Fn(usize) + Send + Sync + 'static,
) -> Result<usize, anyhow::Error> {
    let final_url = get_final_url(url).await?;

    let supports_range = check_range_support(&final_url).await.unwrap_or(false);
    if !supports_range {
        return single_threaded_download(url, target, on_progress).await;
    }

    let total_size = get_content_length(&final_url).await?;
    if total_size == 0 {
        return single_threaded_download(url, target, on_progress).await;
    }

    let chunk_size = total_size / chunk_count as u64;
    let progress_callback = Arc::new(on_progress);
    let total_downloaded = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let target_file = tokio::fs::File::create(target).await?;
    let shared_file = Arc::new(Mutex::new(target_file));

    let mut tasks = Vec::new();

    for i in 0..chunk_count {
        let start = i as u64 * chunk_size;
        let end = if i == chunk_count - 1 {
            total_size - 1
        } else {
            (i + 1) as u64 * chunk_size - 1
        };

        let final_url = final_url.clone();
        let shared_file = Arc::clone(&shared_file);
        let total_downloaded = Arc::clone(&total_downloaded);
        let progress_callback = Arc::clone(&progress_callback);

        let task = tokio::spawn(async move {
            let mut retry_count = 0;
            const MAX_RETRIES: u32 = 3;

            while retry_count < MAX_RETRIES {
                match download_chunk_with_retry(
                    &final_url,
                    start,
                    end,
                    i,
                    &shared_file,
                    &total_downloaded,
                    &progress_callback,
                )
                .await
                {
                    Ok(_) => return Ok::<(), anyhow::Error>(()),
                    Err(e) => {
                        retry_count += 1;
                        if retry_count >= MAX_RETRIES {
                            return Err(anyhow::anyhow!(
                                "Failed to download chunk {} after {} retries: {}",
                                i,
                                MAX_RETRIES,
                                e
                            ));
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            500 * retry_count as u64,
                        ))
                        .await;
                    }
                }
            }

            Ok(())
        });

        tasks.push(task);
    }

    for task in tasks {
        task.await??;
    }

    let total_downloaded = total_downloaded.load(std::sync::atomic::Ordering::Relaxed);

    let mut file = shared_file.lock().await;
    file.flush().await?;
    file.sync_all().await?;
    drop(file);

    Ok(total_downloaded)
}

async fn download_chunk_with_retry(
    url: &str,
    start: u64,
    end: u64,
    chunk_index: usize,
    shared_file: &Arc<Mutex<tokio::fs::File>>,
    total_downloaded: &Arc<std::sync::atomic::AtomicUsize>,
    progress_callback: &Arc<impl Fn(usize) + Send + Sync>,
) -> Result<(), anyhow::Error> {
    let reader = create_http_stream(url, start as usize, (end - start + 1) as usize).await;
    if reader.is_err() {
        return Err(anyhow::anyhow!(
            "Failed to create HTTP stream for chunk {}: {}",
            chunk_index,
            reader.err().unwrap()
        ));
    }
    let mut reader = reader?;
    let mut chunk_data = Vec::new();
    let mut buffer = [0u8; 32768];
    let mut last_progress_time = std::time::Instant::now();

    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => {
                chunk_data.extend_from_slice(&buffer[..n]);
                let current_total =
                    total_downloaded.fetch_add(n, std::sync::atomic::Ordering::Relaxed) + n;

                if last_progress_time.elapsed().as_millis() >= 100 {
                    progress_callback(current_total);
                    last_progress_time = std::time::Instant::now();
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to read chunk {}: {}",
                    chunk_index,
                    e
                ));
            }
        }
    }

    {
        let mut file = shared_file.lock().await;
        file.seek(std::io::SeekFrom::Start(start)).await?;
        file.write_all(&chunk_data).await?;
    }

    Ok(())
}

pub async fn multi_threaded_download(
    url: &str,
    target: &str,
    on_progress: impl Fn(usize) + Send + Sync + 'static,
) -> Result<usize, anyhow::Error> {
    let total_size = get_content_length(url).await.unwrap_or(0);

    if total_size < 1024 * 1024 {
        return single_threaded_download(url, target, on_progress).await;
    }

    let chunk_count = get_optimal_download_threads();

    multi_threaded_download_impl(url, target, chunk_count, on_progress).await
}

pub async fn multi_threaded_download_with_threads(
    url: &str,
    target: &str,
    chunk_count: usize,
    on_progress: impl Fn(usize) + Send + Sync + 'static,
) -> Result<usize, anyhow::Error> {
    let chunk_count = chunk_count.clamp(2, 8);

    multi_threaded_download_impl(url, target, chunk_count, on_progress).await
}

pub async fn single_threaded_download(
    url: &str,
    target: &str,
    on_progress: impl Fn(usize),
) -> Result<usize, anyhow::Error> {
    let source = create_http_stream(url, 0, 0).await?;
    let target_file = create_target_file(target).await?;
    progressed_copy(source, target_file, on_progress).await
}
