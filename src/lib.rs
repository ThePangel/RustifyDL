//!
//! RustifyDL library API
//!
//! High-level helpers to resolve Spotify URLs to tracks, fetch audio from
//! YouTube, and write clean tags to files.
//!
//! Key items:
//! - [`DownloadOptions`] input options
//! - [`download_spotify`] to drive the whole flow asynchronously
//!
//! Examples
//! ```no_run
//! use rustifydl::{download_spotify, DownloadOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let opts = DownloadOptions {
//!         url: "https://open.spotify.com/album/xxxxxxxx".into(),
//!         client_id: "<client_id>".into(),
//!         client_secret: "<client_secret>".into(),
//!         output_dir: "./output".into(),
//!         concurrent_downloads: 6,
//!         no_dupes: true,
//!         bitrate: "192k".into(),
//!         format: "mp3".into(),
//!         verbosity: "info".into(),
//!         no_tag: false,
//!         timeout: 180,
//!     };
//!     download_spotify(opts).await
//! }
//! ```

#![allow(clippy::module_inception)]
use {
    crate::{
        metadata::metadata,
        spotify::{fetch_album, fetch_playlist, fetch_track},
        youtube::{DownloadResult, search_yt},
    },
    env_logger,
    indicatif::{MultiProgress, ProgressBar, ProgressStyle},
    indicatif_log_bridge::LogWrapper,
    log::{error, info},
    regex::Regex,
    spotify_rs::model::track::Track,
    std::{
        collections::HashMap,
        fs,
        io::Write,
        path::PathBuf,
        sync::Arc,
        time::{Duration, Instant},
    },
    tokio::sync::Semaphore,
};

pub mod metadata;
pub mod spotify;
pub mod youtube;

/// Options used to control how downloads are performed.
///
/// These map to CLI flags in the binary.
pub struct DownloadOptions {
    /// Spotify URL (track/album/playlist)
    pub url: String,
    /// Spotify Client ID
    pub client_id: String,
    /// Spotify Client Secret
    pub client_secret: String,
    /// Destination folder for audio files
    pub output_dir: String,
    /// Maximum number of concurrent downloads
    pub concurrent_downloads: usize,
    /// Skip duplicate tracks across collections
    pub no_dupes: bool,
    /// Target audio bitrate for ffmpeg (e.g., "192k")
    pub bitrate: String,
    /// Output format/extension (e.g., "mp3")
    pub format: String,
    /// Log verbosity: one of "full", "debug", "info", "none"
    pub verbosity: String,
    /// Don't write audio tags or cover art
    pub no_tag: bool,
    /// Per-download timeout in seconds for YouTube download
    pub timeout: u64,
}

fn sanitize_filename(name: &str) -> String {
    let re = Regex::new(r#"[<>:"/\\|?*\x00-\x1F]"#).unwrap();
    re.replace_all(name.trim(), "").to_string()
}

/// Extract a Spotify ID from a typical Spotify URL.
///
/// Supports `track/`, `album/`, `playlist/`, and `artist/` URL shapes.
/// Returns `Some(id)` when an ID is present; otherwise `None`.
///
/// Example
/// ```
/// use rustifydl::extract_id_from_url;
/// let id = extract_id_from_url("https://open.spotify.com/track/3n3Ppam7vgaVa1iaRUc9Lp");
/// assert!(id.is_some());
/// ```
pub fn extract_id_from_url(url: &str) -> Option<String> {
    let re = Regex::new(r"(track|album|playlist|artist)/([a-zA-Z0-9]+)").unwrap();

    if let Some(captures) = re.captures(url) {
        return captures.get(2).map(|id| id.as_str().to_string());
    }

    None
}

const SPOTIFY_PATTERNS: [&str; 3] = [
    r"^https://open\.spotify\.com/(track|album|playlist|artist)/.+",
    r"^spotify:(track|album|playlist|artist):.+",
    r"^https://spotify\.link/.+",
];

enum SpotifyUrlType {
    Track,
    Album,
    Playlist,
    Artist,
}

fn is_valid_spotify_url(url: &str) -> Option<(SpotifyUrlType, String)> {
    for pattern in SPOTIFY_PATTERNS.iter() {
        let re = Regex::new(pattern).unwrap();
        if re.is_match(url.trim()) {
            let id = extract_id_from_url(url)?;
            if url.contains("track") {
                return Some((SpotifyUrlType::Track, id));
            } else if url.contains("album") {
                return Some((SpotifyUrlType::Album, id));
            } else if url.contains("playlist") {
                return Some((SpotifyUrlType::Playlist, id));
            } else if url.contains("artist") {
                error!("You wouldn't download an Artist!");
                return Some((SpotifyUrlType::Artist, id));
            }
        }
    }
    None
}

/// Resolve a Spotify URL and download all corresponding tracks.
///
/// Steps:
/// 1. Determine URL type (track/album/playlist) and fetch tracks from Spotify.
/// 2. For each track, search YouTube and download best audio stream.
/// 3. Optionally write tags and artwork (`no_tag == false`).
///
/// Example
/// ```no_run
/// use rustifydl::{download_spotify, DownloadOptions};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// let opts = DownloadOptions {
///     url: "https://open.spotify.com/playlist/xxxxxxxx".into(),
///     client_id: "<client_id>".into(),
///     client_secret: "<client_secret>".into(),
///     output_dir: "./output".into(),
///     concurrent_downloads: 8,
///     no_dupes: true,
///     bitrate: "192k".into(),
///     format: "mp3".into(),
///     verbosity: "info".into(),
///     no_tag: false,
///     timeout: 180,
/// };
/// download_spotify(opts).await?;
/// # Ok(())
/// # }
/// ```
pub async fn download_spotify(
    options: DownloadOptions,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let multi = MultiProgress::new();
    let start_time = Instant::now();
    let mut logger = match options.verbosity.clone().as_str() {
        "full" => {
            let mut builder = env_logger::Builder::new();
            builder
                .format(|buf, record| writeln!(buf, "{}", record.args()))
                .filter_level(log::LevelFilter::Trace);
            builder
        }
        "info" => {
            let mut builder = env_logger::Builder::new();
            builder
                .format(|buf, record| writeln!(buf, "{}", record.args()))
                .filter_level(log::LevelFilter::Off)
                .filter_module("rustifydl", log::LevelFilter::Info);
            builder
        }
        "debug" => {
            let mut builder = env_logger::Builder::new();
            builder.filter_level(log::LevelFilter::Debug);
            builder
        }
        "none" => {
            let mut builder = env_logger::Builder::new();
            builder
                .format(|buf, record| writeln!(buf, "{}", record.args()))
                .filter_level(log::LevelFilter::Off);
            builder
        }
        _ => {
            let mut builder = env_logger::Builder::new();
            builder
                .format(|buf, record| writeln!(buf, "{}", record.args()))
                .filter_level(log::LevelFilter::Info)
                .filter_module("spotify_rs", log::LevelFilter::Warn)
                .filter_module("rustypipe_downloader", log::LevelFilter::Warn);
            builder
        }
    };
    let logger = logger.build();
    LogWrapper::new(multi.clone(), logger).try_init().unwrap();
    let (url_type, id) = is_valid_spotify_url(&options.url).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid Spotify URL")
    })?;

    let tracks = match url_type {
        SpotifyUrlType::Track => fetch_track(&id, &options).await?,
        SpotifyUrlType::Album => fetch_album(&id, &options).await?,
        SpotifyUrlType::Playlist => fetch_playlist(&id, &options).await?,
        SpotifyUrlType::Artist => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Artist URLs are not supported. Please provide a track, album, or playlist URL.",
            )));
        }
    };
    let final_mult = multi.clone();
    download_and_tag_tracks(tracks, &options, multi).await?;
    let bar = final_mult.add(ProgressBar::new(100));
    bar.set_style(ProgressStyle::with_template("{msg}")?);
    bar.finish_with_message(format!("Took {}s", start_time.elapsed().as_secs()));
    Ok(())
}

async fn download_and_tag_tracks(
    tracks: HashMap<String, Track>,
    options: &DownloadOptions,
    multi: MultiProgress,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut handles = Vec::new();
    let semaphore = Arc::new(Semaphore::new(options.concurrent_downloads));
    let lenght = tracks.clone().len();
    let options_cloned = Arc::new(DownloadOptions {
        url: options.url.clone(),
        client_id: options.client_id.to_string().clone(),
        client_secret: options.client_secret.to_string().clone(),
        output_dir: options.output_dir.to_string(),
        concurrent_downloads: options.concurrent_downloads,
        no_dupes: options.no_dupes,
        bitrate: options.bitrate.clone(),
        format: options.format.clone(),
        verbosity: options.verbosity.clone(),
        no_tag: options.no_tag,
        timeout: options.timeout,
    });

    let multi = Arc::new(multi);

    for (i, (name, track)) in tracks.iter().enumerate() {
        let semaphore = semaphore.clone();
        let name = sanitize_filename(&name.as_str());
        let track = track.clone();
        let options_cloned = Arc::clone(&options_cloned);
        let multi = Arc::clone(&multi);
        let handle = tokio::spawn(async move {
            let bar = multi.add(ProgressBar::new_spinner());
            bar.set_style(ProgressStyle::with_template("{spinner:.cyan} {msg}")?);
            bar.enable_steady_tick(Duration::from_millis(100));

            let _permit = semaphore.acquire().await.unwrap();
            bar.set_message(format!("{}/{} Downloading: {}", i + 1, lenght, name));
            if let DownloadResult::Completed = search_yt(&name, options_cloned.as_ref()).await? {
                if !options_cloned.no_tag {
                    bar.set_message(format!("{}/{} Tagging: {}", i + 1, lenght, name));
                    metadata(&name, &track, options_cloned.as_ref()).await?;
                }
            } else {
                bar.finish_with_message(format!("File already exists, skipping!: {}", name));
                return Ok::<(), Box<dyn std::error::Error + Send + Sync>>(());
            }
            bar.finish_with_message(format!("Finished {}!", name));
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
        handles.push(handle);
    }

    for handle in handles {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => error!("Task failed: {}", e),
            Err(e) => error!("Join error: {}", e),
        }
    }
    if PathBuf::from(format!("{}/temp", options.output_dir)).exists() {
        fs::remove_dir_all(format!("{}/temp", options.output_dir))?;
    };

    info!("Finished!");

    Ok(())
}
