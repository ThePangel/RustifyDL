use {
    crate::{
        metadata::metadata,
        spotify::{fetch_album, fetch_playlist, fetch_track},
        youtube::{DownloadResult, search_yt},
    },
    regex::Regex,
    spotify_rs::model::track::Track,
    std::{collections::HashMap, fs, sync::Arc},
    tokio::sync::Semaphore,
};

pub mod metadata;
pub mod spotify;
pub mod youtube;

pub struct DownloadOptions {
    pub url: String,
    pub client_id: String,
    pub client_secret: String,
    pub output_dir: String,
    pub concurrent_downloads: usize,
    pub no_dupes: bool,
}

fn sanitize_filename(name: &str) -> String {
    let re = Regex::new(r#"[<>:"/\\|?*\x00-\x1F]"#).unwrap();
    re.replace_all(name.trim(), "").to_string()
}

pub(crate) fn extract_id_from_url(url: &str) -> Option<String> {
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
                eprintln!("You wouldn't download an Artist!");
                return Some((SpotifyUrlType::Artist, id));
            }
        }
    }
    None
}

pub async fn download_spotify(
    options: DownloadOptions,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    download_and_tag_tracks(tracks, &options).await?;
    Ok(())
}

async fn download_and_tag_tracks(
    tracks: HashMap<String, Track>,
    options: &DownloadOptions,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut handles = Vec::new();
    let semaphore = Arc::new(Semaphore::new(options.concurrent_downloads));
    let lenght = tracks.clone().len();
    for (i, (name, track)) in tracks.iter().enumerate() {
        let semaphore = semaphore.clone();
        let name = sanitize_filename(&name.as_str());
        let track = track.clone();
        let client_id = options.client_id.to_string();
        let client_secret = options.client_secret.to_string();
        let output_dir = options.output_dir.to_string();
        let no_dupes = options.no_dupes;
        let options_cloned = DownloadOptions {
            url: options.url.clone(),
            client_id: client_id.clone(),
            client_secret: client_secret.clone(),
            output_dir,
            concurrent_downloads: options.concurrent_downloads,
            no_dupes,
        };
        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            println!("{}/{} Starting download: {}", i, lenght, name);
            if let DownloadResult::Completed = search_yt(&name, &options_cloned).await? {
                metadata(&name, &track, &options_cloned).await?;
            }

            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
        handles.push(handle);
    }

    for handle in handles {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!("Task failed: {}", e),
            Err(e) => eprintln!("Join error: {}", e),
        }
    }
    fs::remove_dir_all(format!("{}/temp", options.output_dir))?;
    println!("Finished!");
    Ok(())
}
