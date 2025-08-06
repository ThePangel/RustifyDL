use {
    crate::metadata::metadata,
    crate::spotify::{fetch_album, fetch_playlist, fetch_track},
    crate::youtube::search_yt,
    regex::Regex,
    spotify_rs::model::track::Track,
    std::collections::HashMap,
};

pub mod metadata;
pub mod spotify;
pub mod youtube;

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
    url: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (url_type, id) = is_valid_spotify_url(url).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid Spotify URL")
    })?;

    match url_type {
        SpotifyUrlType::Track => {
            let tracks = fetch_track(&id, client_id, client_secret).await?;
            download_and_tag_tracks(tracks, client_id, client_secret).await?;
            return Ok(());
        }
        SpotifyUrlType::Album => {
            let tracks = fetch_album(&id, client_id, client_secret).await?;
            download_and_tag_tracks(tracks, client_id, client_secret).await?;
            return Ok(());
        }
        SpotifyUrlType::Playlist => {
            let tracks = fetch_playlist(&id, client_id, client_secret).await?;
            download_and_tag_tracks(tracks, client_id, client_secret).await?;
            return Ok(());
        }
        SpotifyUrlType::Artist => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Artist URLs are not supported. Please provide a track, album, or playlist URL.",
            )));
        }
    }
}

async fn download_and_tag_tracks(
    tracks: HashMap<String, Track>,
    client_id: &str,
    client_secret: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut handles = Vec::new();

    for (name, track) in &tracks {
        let name = name.clone();
        let track = track.clone();
        let client_id = client_id.to_string();
        let client_secret = client_secret.to_string();
        let handle = tokio::spawn(async move {
            search_yt(&name).await.unwrap();
            metadata(&name, &track, &client_id, &client_secret).await.unwrap();
            
        });
        handles.push(handle);
    }

    println!("Downloading {} tracks...", handles.len());
    for handle in handles {
        handle.await?;
    }
    println!("Finished!");
    Ok(())
}
