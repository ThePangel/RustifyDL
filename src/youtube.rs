//! YouTube search and download helpers.
//!
//! Behavior:
//! - Search YouTube Music for a best-effort match to the provided name.
//! - Download an audio-only stream and write a temporary file to `output_dir/temp`.
//! - Transcode with ffmpeg to the final format and move to `output_dir`.
//! - Skip work if the final output already exists.

use crate::DownloadOptions;

use clap::error::Result;
use hex;
use log::info;
use rustypipe::client::RustyPipe;
use sha2::digest::generic_array::GenericArray;
use sha2::{Digest, Sha256};
use std::fs::{File, remove_file};
use std::io::{BufRead, BufReader, copy};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::{env, fs};
use toml::Value;

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
    ytdlp_path: PathBuf,
) -> Result<DownloadResult, Box<dyn std::error::Error + Send + Sync>> {
    let rp = RustyPipe::new();
    let search_results = rp.query().music_search_tracks(name).await?;

    if let DownloadResult::Skipped = download(
        search_results.items.items[0].id.as_str(),
        name,
        options,
        ytdlp_path,
    )
    .await?
    {
        return Ok(DownloadResult::Skipped);
    }
    Ok(DownloadResult::Completed)
}

/// Download by YouTube video id and transcode to the target format using ffmpeg.
///
/// The temporary file is saved under `output_dir/temp/`
pub async fn download(
    id: &str,
    name: &str,
    options: &DownloadOptions,
    ytdlp_path: PathBuf,
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

    let fixed_path = if ytdlp_path.is_relative() && !ytdlp_path.starts_with(".") {
        PathBuf::from(".").join(ytdlp_path)
    } else {
        ytdlp_path
    };
    let download_video = Command::new(fixed_path.to_str().ok_or("Invalid UTF-8 in file path")?)
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

    command_error_print(download_video)?;

    file = PathBuf::from(format!(
        "{}.opus",
        file.to_str().ok_or("Invalid UTF-8 in file path")?
    ));
    if file.exists() {
        transcode(
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
fn transcode(
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

    command_error_print(output)?;

    info!("Completed: {name}");
    Ok(())
}

/// Downloads the latest ytdlpd binary for the users OS
/// and gives the current user executing permissions (Linux & MacOS)
/// Also handles custom download directories through the config file if needed
pub fn download_ytdlp(
    ytdlp_dir: String,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let mut ytdlp_path;

    let config_dir = dirs::config_dir().ok_or("Could not find a valid config directory.")?;
    let app_config_dir = config_dir.join("RustifyDL");

    let config_path = app_config_dir.join("config.toml");

    if config_path.exists() && config_path.is_file() && fs::metadata(&config_path)?.len() != 0 {
        let content = fs::read_to_string(&config_path)?;
        let mut config_file = toml::from_str::<Value>(&content)?;
        ytdlp_path = match config_file["ytdlp_dir"].clone().as_str() {
            Some(a) => {
                if !ytdlp_dir.is_empty() {
                    if ytdlp_dir == "default" {
                        config_file["ytdlp_dir"] = Value::String("".to_string());
                        fs::write(
                            config_path,
                            toml::to_string(&config_file).expect("Failed to serialize TOML"),
                        )?;
                        app_config_dir
                    } else {
                        config_file["ytdlp_dir"] = Value::String(ytdlp_dir.clone());
                        fs::write(
                            config_path,
                            toml::to_string(&config_file).expect("Failed to serialize TOML"),
                        )?;
                        PathBuf::from(a)
                    }
                } else if !a.is_empty() {
                    PathBuf::from(a)
                } else {
                    app_config_dir
                }
            }
            None => app_config_dir,
        };
        fs::create_dir_all(ytdlp_dir)?;
    } else {
        ytdlp_path = app_config_dir;
    }
    if env::consts::OS == "windows" {
        ytdlp_path = ytdlp_path.join("yt-dlp.exe");
        if !ytdlp_path.exists() {
            println!("Downloading yt-dlp binary (First time only or update/repair)");
            let curl = Command::new("curl")
                .args([
                    "-L",
                    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
                    "-o",
                    ytdlp_path.to_str().ok_or("Invalid UTF-8 in file path")?,
                ])
                .output()?;
            command_error_print(curl)?;
        } else {
            update_ytdlp(ytdlp_path.clone())?;
        }
    } else {
        ytdlp_path = ytdlp_path.join("yt-dlp");

        if !ytdlp_path.exists() {
            println!("Downloading yt-dlp binary (First time only or update/repair)");
            match env::consts::OS {
                "linux" => {
                    let curl = Command::new("curl").args([
                        "-L",
                        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux",
                        "-o",
                        ytdlp_path.to_str().ok_or("Invalid UTF-8 in file path")?,
                    ]).output()?;
                    command_error_print(curl)?;

                    let chmod = Command::new("chmod")
                        .args([
                            "a+rx",
                            ytdlp_path.to_str().ok_or("Invalid UTF-8 in file path")?,
                        ])
                        .output()?;
                    command_error_print(chmod)?;
                }
                "macos" => {
                    let curl = Command::new("curl").args([
                        "-L",
                        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos",
                        "-o",
                        ytdlp_path.to_str().ok_or("Invalid UTF-8 in file path")?,
                    ]).output()?;

                    command_error_print(curl)?;
                    let chmod = Command::new("chmod")
                        .args([
                            "a+rx",
                            ytdlp_path.to_str().ok_or("Invalid UTF-8 in file path")?,
                        ])
                        .output()?;
                    command_error_print(chmod)?;
                }
                _ => {}
            }
        } else {
            update_ytdlp(ytdlp_path.clone())?;
        }
    }
    Ok(ytdlp_path)
}

/// Compares latest ytdlp checksum to the installed binary's checksum
/// to update or repair the binary
pub fn update_ytdlp(
    mut ytdlp_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let curl = Command::new("curl")
        .args([
            "-L",
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS",
            "-o",
            "./checksums",
        ])
        .output()?;
    command_error_print(curl)?;
    let checksums_file = File::open(Path::new("./checksums"))?;
    let reader = BufReader::new(checksums_file.try_clone()?);
    let mut checksum = String::new();
    if env::consts::OS == "linux" {
        if let Some(Ok(line)) = reader.lines().nth(4) {
            checksum = String::from(
                line.split_whitespace()
                    .next()
                    .ok_or("Couldn't read checksum")?,
            );
        }
    } else if env::consts::OS == "macos" {
        if let Some(Ok(line)) = reader.lines().nth(9) {
            checksum = String::from(
                line.split_whitespace()
                    .next()
                    .ok_or("Couldn't read checksum")?,
            );
        }
    } else if env::consts::OS == "windows"
        && let Some(Ok(line)) = reader.lines().nth(2)
    {
        checksum = String::from(
            line.split_whitespace()
                .next()
                .ok_or("Couldn't read checksum")?,
        );
    }
    let mut ytdlp_file = File::open(ytdlp_path.clone())?;

    let mut sha256 = Sha256::new();
    copy(&mut ytdlp_file, &mut sha256)?;
    let hash = sha256.finalize();

    let checksum_decoded = hex::decode(&checksum)?;
    remove_file("./checksums")?;
    if hash != GenericArray::clone_from_slice(&checksum_decoded) {
        remove_file(&ytdlp_path)?;
        ytdlp_path.pop();
        download_ytdlp(ytdlp_path.into_os_string().into_string().unwrap())?;
    }
    Ok(())
}

/// Handles error correction on commands
fn command_error_print(command: Output) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !command.status.success() {
        return Err(Box::new(std::io::Error::other(format!(
            "Command error: {}",
            String::from_utf8_lossy(&command.stderr)
        ))));
    }
    Ok(())
}
