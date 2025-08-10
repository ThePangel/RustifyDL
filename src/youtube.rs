//! YouTube search and download helpers.
//!
//! Behavior:
//! - Search YouTube Music for a best-effort match to the provided name.
//! - Download an audio-only stream and write a temporary file to `output_dir/temp`.
//! - Transcode with ffmpeg to the final format and move to `output_dir`.
//! - Skip work if the final output already exists.

use crate::DownloadOptions;
use log::info;
use rustypipe::client::RustyPipe;
use rustypipe::param::StreamFilter;
use rustypipe_downloader::DownloaderBuilder;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;

/// Result of a download attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadResult {
    /// File was downloaded and processed successfully.
    Completed,
    /// File already existed and was skipped.
    Skipped,
}

/// Search YouTube Music for `name` and calls the `rustifydl::youtube::download` fuction to download the video.
pub async fn search_yt(
    name: &str,
    options: &DownloadOptions,
) -> Result<DownloadResult, Box<dyn std::error::Error + Send + Sync>> {
    let rp = RustyPipe::new();
    let search_results = rp.query().music_search_tracks(name).await?;

    if let DownloadResult::Skipped =
        download(search_results.items.items[0].id.as_str(), name, options).await?
    {
        return Ok(DownloadResult::Skipped);
    }
    Ok(DownloadResult::Completed)
}

/// Download by YouTube video id and transcode to the target format using ffmpeg.
///
/// The temporary file is saved under `output_dir/temp/` and removed on timeout
/// or failure.
pub async fn download(
    id: &str,
    name: &str,
    options: &DownloadOptions,
) -> Result<DownloadResult, Box<dyn std::error::Error + Send + Sync>> {
    fs::create_dir_all(options.output_dir.clone())?;

    let dl = DownloaderBuilder::new().build();
    let filter_audio = StreamFilter::new().no_video();
    let mut file = PathBuf::from(format!("{}/temp/{}", options.output_dir, name));
    let processed_file = PathBuf::from(format!(
        "{}/{}.{}",
        options.output_dir,
        name,
        options.format.clone()
    ));
    if processed_file.exists() {
        info!("File already exists, skipping: {}", name);
        return Ok(DownloadResult::Skipped);
    }

    let download_builder = dl.id(id).stream_filter(filter_audio).to_file(&file);
    let download_status = download_builder.download();

    match timeout(Duration::from_secs(options.timeout), download_status).await {
        Ok(inner_result) => {
            if let Ok(value) = inner_result {
                file = value.dest;
            } else if let Err(e) = inner_result {
                return Err(format!("Download library error for {}: {}", name, e).into());
            }
        }
        Err(_) => {
            if std::path::Path::new(&file).exists() {
                let _ = fs::remove_file(&file);
            }
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!(
                    "Download for {} timed out after {} seconds",
                    name, options.timeout
                ),
            )));
        }
    }
    if file.exists() {
        convert_to_mp3(
            file.to_str().ok_or("Invalid UTF-8 in file path")?,
            processed_file
                .to_str()
                .ok_or("Invalid UTF-8 in file path")?,
            name,
            &options,
        )?;
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidFilename,
            format!(
                "Download for {} failed or didn't start: File not Found",
                name
            ),
        )));
    }

    Ok(DownloadResult::Completed)
}

/// Transcode the intermediate download to the desired output format using ffmpeg.
///
/// Uses `-b:a <bitrate>` and `-threads 0` to allow ffmpeg to use all cores. On
/// failure, the stderr from ffmpeg is surfaced in the error.
fn convert_to_mp3(
    input_file: &str,
    output_file: &str,
    name: &str,
    options: &DownloadOptions,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("ffmpeg")
        .args([
            "-i",
            input_file,
            "-b:a",
            &options.bitrate,
            "-threads",
            "0",
            "-y",
            output_file,
        ])
        .output()?;

    if !output.status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "FFmpeg conversion failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        )));
    }

    info!("Completed: {}", name);
    Ok(())
}
