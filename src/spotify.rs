//! Spotify helpers for resolving tracks, albums, and playlists.
//!
//! Input: a Spotify ID string and authenticated client credentials (provided
//! via [`DownloadOptions`]).
//! Output: a `HashMap<String, Track>` keyed by a human-friendly display name,
//! e.g. `"Artists - Title"` or with dupes, if there are two of the same file `"Artists - Album - Title"`.

use spotify_rs::model::track::Track;
use spotify_rs::{ClientCredsClient, model::PlayableItem};
use std::collections::HashMap;
use crate::DownloadOptions;
use log::info;

/// Fetch a single track by Spotify ID.
///
/// Returns a map with one entry mapping a display name to its `spotify_rs::model::track::Track`.
pub async fn fetch_track(
    id: &str,
    options: &DownloadOptions,
) -> Result<HashMap<String, Track>, Box<dyn std::error::Error + Send + Sync>> {
    let spotify = ClientCredsClient::authenticate(&options.client_id, &options.client_secret).await?;
    let track = spotify_rs::track(id).get(&spotify).await?;
    let mut songs = HashMap::<String, Track>::new();
    songs.insert(
        format!(
            "{} - {}",
            track
                .artists
                .iter()
                .map(|artist| artist.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            track.name,
        ),
        track,
    );
    Ok(songs)
}

/// Fetch all tracks from a playlist by ID.
///
/// The result map keys are display names. If `DownloadOptions::no_dupes` is
/// false and a duplicate title is encountered, the album name is appended to
/// disambiguate entries.
/// Returns a HashMap with the track name as the key and the `` object. 

pub async fn fetch_playlist(
    id: &str,
    options: &DownloadOptions,
) -> Result<HashMap<String, Track>, Box<dyn std::error::Error + Send + Sync>> {
    let spotify = ClientCredsClient::authenticate(&options.client_id, &options.client_secret).await?;
    let mut songs = HashMap::<String, Track>::new();

    let playlist = spotify_rs::playlist(id).get(&spotify).await?;
    let tracks = playlist.tracks.get_all(&spotify).await?;
    for song in tracks {
        if let Some(song) = song {
            match song.track {
                PlayableItem::Track(track) => {
                    if songs.contains_key(
                        format!(
                            "{} - {}",
                            track
                                .artists
                                .iter()
                                .map(|artist| artist.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", "),
                            track.name,
                        )
                        .as_str(),
                    ) && !options.no_dupes {
                        songs.insert(
                            format!(
                                "{} - {} - {}",
                                track
                                    .artists
                                    .iter()
                                    .map(|artist| artist.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                track.album.name,
                                track.name,
                            ),
                            track,
                        );
                    } else {
                        songs.insert(
                            format!(
                                "{} - {}",
                                track
                                    .artists
                                    .iter()
                                    .map(|artist| artist.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                track.name,
                            ),
                            track,
                        );
                    }
                }
                PlayableItem::Episode(_episode) => {}
            }
        } else {
            info!("No song found.");
        }
    }
    info!("Found {} tracks in {}!", songs.len(), playlist.name);
    Ok(songs)
}

/// Fetch all tracks from a Album by ID.
///
/// The result map keys are display names. If `DownloadOptions::no_dupes` is
/// false and a duplicate title is encountered, the album name is appended to
/// disambiguate entries.
/// Returns a HashMap with the track name as the key and the `spotify_rs::model::track::Track` object. 
pub async fn fetch_album(
    id: &str,
    options: &DownloadOptions,
) -> Result<HashMap<String, Track>, Box<dyn std::error::Error + Send + Sync>> {
    let spotify = ClientCredsClient::authenticate(&options.client_id, &options.client_secret).await?;
    let mut songs = HashMap::<String, Track>::new();

    let album = spotify_rs::album(id).get(&spotify).await?;
    let tracks = album.tracks.get_all(&spotify).await?;

    for song in tracks {
        if let Some(song) = song {
            let track = spotify_rs::track(song.id).get(&spotify).await?;
            songs.insert(
                format!(
                    "{} - {}",
                    track
                        .artists
                        .iter()
                        .map(|artist| artist.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    track.name,
                ),
                track,
            );
        } else {
            info!("No song found.");
        }
    }
    info!("Found {} tracks in {}!", songs.len(), album.name);
    Ok(songs)
}
