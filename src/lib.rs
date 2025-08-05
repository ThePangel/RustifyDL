use {
    crate::metadata::metadata,
    crate::spotify::{fetch_album, fetch_playlist, fetch_track},
    crate::youtube::search_yt,
    regex::Regex,
    std::collections::HashMap,
    spotify_rs::model::track::Track,
};

pub mod metadata;
pub mod spotify;
pub mod youtube;

pub async fn download_spotify(
    url: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match is_valid_spotify_url(url) {
        Some(SpotifyUrlType::Track) => {
            let tracks = fetch_track(url, client_id, client_secret).await?;
            download_and_tag_tracks(tracks, client_id, client_secret).await?;
        }
        Some(SpotifyUrlType::Album) => {
            let tracks = fetch_album(url, client_id, client_secret).await?;
            download_and_tag_tracks(tracks, client_id, client_secret).await?;
        }
        Some(SpotifyUrlType::Playlist) => {
            let tracks = fetch_playlist(url, client_id, client_secret).await?;
            download_and_tag_tracks(tracks, client_id, client_secret).await?;
        }
        Some(SpotifyUrlType::Artist) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Artist URLs are not supported. Please provide a track, album, or playlist URL.",
            )));
        }
        None => println!("Invalid Spotify URL."),
        _ => println!("Unknown type."),
    }
    Ok(())
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

fn is_valid_spotify_url(url: &str) -> Option<SpotifyUrlType> {
    for pattern in SPOTIFY_PATTERNS.iter() {
        let re = Regex::new(pattern).unwrap();
        if re.is_match(url.trim()) {
            if url.contains("track") {
                return Some(SpotifyUrlType::Track);
            } else if url.contains("album") {
                return Some(SpotifyUrlType::Album);
            } else if url.contains("playlist") {
                return Some(SpotifyUrlType::Playlist);
            } else if url.contains("artist") {
                eprintln!("You wouldn't download an Artist!");
                return Some(SpotifyUrlType::Artist);
            }
        }
    }
    None
}

async fn download_and_tag_tracks(
    tracks: HashMap<String, Track>,
    client_id: &str,
    client_secret: &str
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, _track) in &tracks {
        search_yt(name.as_str()).await?;
    }
    metadata(tracks, client_id, client_secret).await?;
    Ok(())
}
