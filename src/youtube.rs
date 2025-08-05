use std::fs;
use std::path::PathBuf;
use yt_dlp::Youtube;
use yt_dlp::fetcher::deps::Libraries;
use yt_dlp::fetcher::deps::LibraryInstaller;
use yt_search::{Duration, SearchFilters, SortBy, YouTubeSearch};

pub(crate) async fn search(name: &str) {
    let search = match YouTubeSearch::new(None, false) {
        Ok(search) => search,
        Err(e) => {
            eprintln!("Failed to initialize YouTubeSearch: {}", e);
            return;
        }
    };
    let filters = SearchFilters {
        sort_by: Some(SortBy::ViewCount),
        duration: Some(Duration::Long),
    };

    match search.search(name, filters).await {
        Ok(results) => {
            download(results[0].video_id.as_str()).await.unwrap();
        }
        Err(e) => eprintln!("Search error: {}", e),
    }
}

async fn download(id: &str) -> Result<(), Box<dyn std::error::Error>> {
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
        PathBuf::from(program_data)
            .join("RustifyDL")
            .join("output/songs")
    } else if cfg!(target_os = "linux") {
        PathBuf::from("/usr/local/share/RustifyDL/output/songs")
    } else {
        PathBuf::from("output/songs")
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
    fetcher
        .download_format(&audio_format, format!("{}.mp3", video.title))
        .await?;

    Ok(())
}
