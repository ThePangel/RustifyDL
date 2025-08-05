use yt_dlp::Youtube;
use std::path::{Path, PathBuf};
use yt_dlp::fetcher::deps::Libraries;
use std::fs;
use yt_dlp::fetcher::deps::LibraryInstaller;

pub(crate) async fn download(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    
    let libraries_dir = if cfg!(target_os = "windows") {
        let program_data = std::env::var("PROGRAMDATA")?;
        PathBuf::from(program_data).join("RustifyDL").join("libs")
    } else if cfg!(target_os = "linux") {
        PathBuf::from("/usr/local/share/RustifyDL/libs")
    } else {
        PathBuf::from("libs")
    };

    let output_dir = if cfg!(target_os = "windows") {
        let program_data = std::env::var("PROGRAMDATA")?;
        PathBuf::from(program_data).join("RustifyDL").join("output")
    } else if cfg!(target_os = "linux") {
        PathBuf::from("/usr/local/share/RustifyDL/output")
    } else {
        PathBuf::from("output")
    };

    fs::create_dir_all(&libraries_dir)?;
    fs::create_dir_all(&output_dir)?;

    let mut youtube = libraries_dir.join("yt-dlp");
    let mut ffmpeg = libraries_dir.join("ffmpeg");

   
    if !youtube.exists() {
        println!("yt-dlp not found. Installing...");
        let installer = LibraryInstaller::new(libraries_dir.clone());
        youtube = installer.install_youtube(None).await.unwrap();
    } 

    if !ffmpeg.exists() {
        println!("ffmpeg not found. Installing...");
        let installer = LibraryInstaller::new(libraries_dir.clone());
        ffmpeg = installer.install_ffmpeg(None).await.unwrap();
    }

    let libraries = Libraries::new(youtube, ffmpeg);
    let fetcher = Youtube::new(libraries, output_dir)?;

    fetcher.update_downloader().await?;
    let url = String::from(id);
    let video = fetcher.fetch_video_infos(url).await?;

    let audio_format = video.worst_audio_format().unwrap();
    let audio_path = fetcher
        .download_format(&audio_format, format!("{}.mp3", video.title))
        .await?;

    println!("Audio downloaded to: {:?}", audio_path);

    Ok(())
}
