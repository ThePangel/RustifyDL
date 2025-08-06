use rustypipe::client::RustyPipe;
use rustypipe::param::StreamFilter;
use rustypipe_downloader::DownloaderBuilder;
use std::fs;
use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;

pub(crate) async fn search_yt(name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rp = RustyPipe::new();
    let search_results = rp.query().music_search_tracks(name).await?;

    download(search_results.items.items[0].id.as_str(), name).await?;
    Ok(())
}

async fn download(id: &str, name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fs::create_dir_all("./output")?;

    let dl = DownloaderBuilder::new().build();
    let filter_audio = StreamFilter::new().no_video();
    let mut file = format!("./output/{}", name);
    let processed_file = format!("./output/{}.mp3", name);
    if std::path::Path::new(&processed_file).exists() {
        println!("File already exists, skipping: {}", name);
        return Ok(());
    }
    println!("Starting download: {}", name);
    let download_builder = dl.id(id).stream_filter(filter_audio).to_file(&file);
    let download_status = download_builder.download();

    match timeout(Duration::from_secs(180), download_status).await {
        Ok(inner_result) => {
            if let Ok(value) = inner_result {
                file = value
                    .dest
                    .to_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Failed to convert destination path to string for {}", name),
                    ))?;
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
    if std::path::Path::new(&file).exists(){
        convert_to_mp3(&file.as_str(), &processed_file.as_str(), name)?;
    } else {
        return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidFilename,
                format!("Download for {} failed or didn't start: File not Found", name),
            )));
    }

    Ok(())
}

fn convert_to_mp3(
    input_file: &str,
    output_file: &str,
    name: &str
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("ffmpeg")
        .args([
            "-i",
            input_file,
            "-c:a",
            "libmp3lame",
            "-preset",
            "ultrafast",
            "-b:a",
            "96k",
            "-q:a",
            "9",
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
    fs::remove_file(input_file)?;
    println!("Completed: {}", name);
    Ok(())
}
