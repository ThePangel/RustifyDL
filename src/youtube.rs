use rustypipe::client::RustyPipe;
use rustypipe::param::StreamFilter;
use rustypipe_downloader::DownloaderBuilder;
use std::path::PathBuf;
use std::{fs};
use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use crate::DownloadOptions;

pub enum DownloadResult {
    Completed,
    Skipped,
}

pub(crate) async fn search_yt(name: &str, options: &DownloadOptions) -> Result<DownloadResult, Box<dyn std::error::Error + Send + Sync>> {
    let rp = RustyPipe::new();
    let search_results = rp.query().music_search_tracks(name).await?;

    download(search_results.items.items[0].id.as_str(), name, options).await?;
    Ok(DownloadResult::Completed)
}

async fn download(id: &str, name: &str, options: &DownloadOptions) -> Result<DownloadResult, Box<dyn std::error::Error + Send + Sync>> {
    fs::create_dir_all(options.output_dir.clone())?;

    let dl = DownloaderBuilder::new().build();
    let filter_audio = StreamFilter::new().no_video();
    let mut file = PathBuf::from(format!("{}/temp/{}", options.output_dir, name));
    let processed_file =PathBuf::from(format!("{}/{}.{}", options.output_dir, name, options.format.clone()));
    if processed_file.exists() {
        println!("File already exists, skipping: {}", name);
        return Ok(DownloadResult::Skipped);
    }
   
    let download_builder = dl.id(id).stream_filter(filter_audio).to_file(&file);
    let download_status = download_builder.download();

    match timeout(Duration::from_secs(180), download_status).await {
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
                format!("Download for {} timed out after 3 minutes", name),
            )));
        }
    }
    if file.exists() {
        convert_to_mp3(file.to_str().ok_or("Invalid UTF-8 in file path")?, processed_file.to_str().ok_or("Invalid UTF-8 in file path")?, name, &options)?;
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
   
    println!("Completed: {}", name);
    Ok(())
}
