use lofty::{
    config::WriteOptions,
    file::TaggedFileExt,
    picture::{MimeType, Picture, PictureType},
    read_from_path,
    tag::{Accessor, Tag, TagExt},
};
use reqwest;
use spotify_rs::{ClientCredsClient, model::track::Track};
use std::{collections::HashMap, path::PathBuf};

pub(crate) async fn metadata(
    songs: HashMap<String, Track>,
    client_id: &str,
    client_secret: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let spotify = ClientCredsClient::authenticate(client_id, client_secret).await?;
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
    for (key, value) in &songs {
        let path = format!("{}/{}.mp3", output_dir.to_string_lossy(), key);
        let mut tagged_file = read_from_path(&path)?;

        let tag = match tagged_file.primary_tag_mut() {
            Some(primary_tag) => primary_tag,
            None => {
                if let Some(first_tag) = tagged_file.first_tag_mut() {
                    first_tag
                } else {
                    let tag_type = tagged_file.primary_tag_type();

                    eprintln!("WARN: No tags found, creating a new tag of type `{tag_type:?}`");
                    tagged_file.insert_tag(Tag::new(tag_type));

                    tagged_file.primary_tag_mut().unwrap()
                }
            }
        };

        tag.set_title(value.name.clone());
        tag.set_artist(
            value
                .artists
                .iter()
                .map(|artist| artist.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
        tag.set_album(value.album.name.clone());
        let album = spotify_rs::album(value.album.id.clone())
            .get(&spotify)
            .await?;
        tag.set_genre(
            album
                .genres
                .iter()
                .map(|genre| genre.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
        tag.set_disk(value.disc_number);

        let image_url = &album.images[0].url;
        let image_bytes = reqwest::get(image_url).await?.bytes().await?.to_vec();

        let front_cover = Picture::new_unchecked(
            PictureType::CoverFront,
            Some(MimeType::Png),
            None,
            image_bytes,
        );
        tag.push_picture(front_cover);
        tag.set_track(value.track_number);
        tag.set_track_total(album.total_tracks);

        if album.release_date.len() >= 4 {
            if let Ok(year) = album.release_date[..4].parse::<u32>() {
                tag.set_year(year);
            }
        }

        tag.save_to_path(path.clone(), WriteOptions::default())
            .expect("ERROR: Failed to write the tag!");
    }
    Ok(())
}
