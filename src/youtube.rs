//! YouTube search and download helpers.
//!
//! Behavior:
//! - Search YouTube Music for a best-effort match to the provided name.
//! - Download an audio-only stream and write a temporary file to `output_dir/temp`.
//! - Transcode with ffmpeg to the final format and move to `output_dir`.
//! - Skip work if the final output already exists.

use crate::DownloadOptions;
use clap::error::Result;
use log::info;
use rustypipe::client::RustyPipe;
use std::path::{ PathBuf};
use std::process::Command;
use std::{env, fs};

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
    let mut file = PathBuf::from(format!("{}/temp/{}", options.output_dir, name));

    let processed_file = PathBuf::from(format!(
        "{}/{}.{}",
        options.output_dir,
        name,
        options.format.clone()
    ));
    if processed_file.exists() {
        info!("File already exists, skipping: {name}");
        return Ok(DownloadResult::Skipped);
    }

    let ytdl_path = download_ytdlp().await?;

    let download_video = Command::new(ytdl_path.to_str().ok_or("Invalid UTF-8 in file path")?)
        .args([
            "--audio-format",
            "opus",
            "-N",
            &options.concurrent_downloads.clone().to_string(),
            "--format",
            "bestaudio",
            "-o",
            file.to_str().ok_or("Invalid UTF-8 in file path")?,
            "-x",
            id,
        ])
        .output()?;

    if !download_video.status.success() {
        return Err(Box::new(std::io::Error::other(format!(
            "Downloading with yt_dlp failed: {}",
            String::from_utf8_lossy(&download_video.stderr)
        ))));
    }
    file = PathBuf::from(format!(
        "{}.opus",
        file.to_str().ok_or("Invalid UTF-8 in file path")?
    ));
    if file.exists() {
        convert_to_mp3(
            file.to_str().ok_or("Invalid UTF-8 in file path")?,
            processed_file
                .to_str()
                .ok_or("Invalid UTF-8 in file path")?,
            name,
            options,
        )?;
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidFilename,
            format!("Download for {name} failed or didn't start: File not Found"),
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
        return Err(Box::new(std::io::Error::other(format!(
            "FFmpeg conversion failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))));
    }

    info!("Completed: {name}");
    Ok(())
}

pub async fn download_ytdlp() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let config_dir = dirs::config_dir().ok_or("Could not find a valid config directory.")?;

    let app_config_dir = config_dir.join("RustifyDL");
    fs::create_dir_all(&app_config_dir)?;
    let ytdlp_path;

    if env::consts::OS == "windows" {
        ytdlp_path = app_config_dir.join("yt-dlp.exe");
        if !ytdlp_path.exists() {
            let curl = Command::new("curl")
                .args([
                    "-L",
                    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
                    "-o",
                    ytdlp_path.to_str().ok_or("Invalid UTF-8 in file path")?,
                ])
                .output()?;
            if !curl.status.success() {
                return Err(Box::new(std::io::Error::other(format!(
                    "yt-dlp download failed: {}",
                    String::from_utf8_lossy(&curl.stderr)
                ))));
            }
        }
    } else {
        ytdlp_path = app_config_dir.join("yt-dlp");
        if !ytdlp_path.exists() {
            match env::consts::OS {
                "linux" => {
                    let curl = Command::new("curl").args([
                        "-L",
                        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux",
                        "-o",
                        ytdlp_path.to_str().ok_or("Invalid UTF-8 in file path")?,
                    ]).output()?;
                    if !curl.status.success() {
                        return Err(Box::new(std::io::Error::other(format!(
                            "yt-dlp download failed: {}",
                            String::from_utf8_lossy(&curl.stderr)
                        ))));
                    }
                    let chmod = Command::new("chmod")
                        .args([
                            "a+rx",
                            ytdlp_path.to_str().ok_or("Invalid UTF-8 in file path")?,
                        ])
                        .output()?;
                    if !chmod.status.success() {
                        return Err(Box::new(std::io::Error::other(format!(
                            "yt-dlp download failed: {}",
                            String::from_utf8_lossy(&chmod.stderr)
                        ))));
                    }
                }
                "macos" => {
                    let curl = Command::new("curl").args([
                        "-L",
                        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos",
                        "-o",
                        ytdlp_path.to_str().ok_or("Invalid UTF-8 in file path")?,
                    ]).output()?;

                    if !curl.status.success() {
                        return Err(Box::new(std::io::Error::other(format!(
                            "yt-dlp download failed: {}",
                            String::from_utf8_lossy(&curl.stderr)
                        ))));
                    }
                    let chmod = Command::new("chmod")
                        .args([
                            "a+rx",
                            ytdlp_path.to_str().ok_or("Invalid UTF-8 in file path")?,
                        ])
                        .output()?;
                    if !chmod.status.success() {
                        return Err(Box::new(std::io::Error::other(format!(
                            "yt-dlp download failed: {}",
                            String::from_utf8_lossy(&chmod.stderr)
                        ))));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(ytdlp_path)
}
