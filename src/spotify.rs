use spotify_rs::{ClientCredsClient, model::PlayableItem};
use std::collections::HashMap;
use spotify_rs::model::track::{Track};

pub(crate) async fn fetch_track(
    id: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<HashMap<String, Track>, Box<dyn std::error::Error>> {
    let spotify = ClientCredsClient::authenticate(client_id, client_secret).await?;

    let track = spotify_rs::track(id).get(&spotify).await?;
    let mut songs = HashMap::<String, Track>::new();
    songs.insert(
        format!(
            "{} - {}",
            track.name,
            track
                .artists
                .iter()
                .map(|artist| artist.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        track,
    );
    Ok(songs)
}

pub(crate) async fn fetch_playlist(
    id: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<HashMap<String, Track>, Box<dyn std::error::Error>> {
    let spotify = ClientCredsClient::authenticate(client_id, client_secret).await?;
    let mut songs = HashMap::<String, Track>::new();

    let playlist = spotify_rs::playlist(id).get(&spotify).await?;
    let tracks = playlist.tracks.get_all(&spotify).await?;
    for song in tracks {
        if let Some(song) = song {
            match song.track {
                PlayableItem::Track(track) => {
                    songs.insert(
                        format!(
                            "{} - {}",
                            track.name,
                            track
                                .artists
                                .iter()
                                .map(|artist| artist.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        track,
                    );
                }
                PlayableItem::Episode(_episode) => {}
            }
        } else {
            println!("No song found.");
        }
    }
    Ok(songs)
}

pub(crate) async fn fetch_album(
    id: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<HashMap<String, Track>, Box<dyn std::error::Error>>
{
    let spotify = ClientCredsClient::authenticate(client_id, client_secret).await?;
    let mut songs = HashMap::<String, Track>::new();
    
    let album = spotify_rs::album(id).get(&spotify).await?;
    let tracks = album.tracks.get_all(&spotify).await?;
    for song in tracks {
        if let Some(song) = song {
            let track = spotify_rs::track(&song.href).get(&spotify).await?;
            songs.insert(
                format!(
                    "{} - {}",
                    track.name,
                    track.artists
                        .iter()
                        .map(|artist| artist.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                track,
            );
        } else {
            println!("No song found.");
        }
    }

    Ok(songs)
}
