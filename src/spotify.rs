use spotify_rs::model::track::Track;
use spotify_rs::{ClientCredsClient, model::PlayableItem};
use std::collections::HashMap;
use crate::DownloadOptions;
use log::info;

pub(crate) async fn fetch_track(
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

pub(crate) async fn fetch_playlist(
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

pub(crate) async fn fetch_album(
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
