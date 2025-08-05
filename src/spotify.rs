use spotify_rs::{ClientCredsClient, model::PlayableItem};

pub(crate) async fn fetch_track(
    id: String,
    client_id: String,
    client_secret: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let spotify = ClientCredsClient::authenticate(client_id, client_secret).await?;

    let track = spotify_rs::track(&id).get(&spotify).await?;

    Ok(format!(
        "{} - {}",
        track.name,
        track
            .artists
            .iter()
            .map(|artist| artist.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

pub(crate) async fn fetch_playlist(
    id: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let spotify = ClientCredsClient::authenticate(client_id, client_secret).await?;
    let mut songs = Vec::<String>::new();

    let playlist = spotify_rs::playlist(id).get(&spotify).await?;
    let tracks = playlist.tracks.get_all(&spotify).await?;
    for song in tracks {
        if let Some(song) = song {
            match song.track {
                PlayableItem::Track(track) => {
                    songs.push(format!(
                        "{} - {}",
                        track.name,
                        track
                            .artists
                            .iter()
                            .map(|artist| artist.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
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
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let spotify = ClientCredsClient::authenticate(client_id, client_secret).await?;
    let mut songs = Vec::<String>::new();

    let album = spotify_rs::album(id).get(&spotify).await?;
    let tracks = album.tracks.get_all(&spotify).await?;
    for song in tracks {
        if let Some(song) = song {
            songs.push(format!(
                "{} - {}",
                song.name,
                song.artists
                    .iter()
                    .map(|artist| artist.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        } else {
            println!("No song found.");
        }
    }

    Ok(songs)
}
