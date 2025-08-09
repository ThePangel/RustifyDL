use lofty::{
    config::WriteOptions,
    file::{AudioFile, TaggedFileExt},
    picture::{MimeType, Picture, PictureType},
    read_from_path,
    tag::{Accessor, Tag},
};
use reqwest;
use spotify_rs::{
    ClientCredsClient,
    model::{track::Track},
};

use crate::DownloadOptions;


fn detect_image_mime_type(bytes: &[u8]) -> MimeType {
    if bytes.len() < 4 {
        return MimeType::Jpeg;
    }

    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return MimeType::Jpeg;
    }

    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return MimeType::Png;
    }

    MimeType::Jpeg
}
pub(crate) async fn metadata(
    song: &String,
    track: &Track,
    options: &DownloadOptions,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let spotify = ClientCredsClient::authenticate(options.client_id.clone(), options.client_secret.clone()).await?;

    let path = format!("{}/{}.{}", options.output_dir, song, options.format.clone());
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

    tag.set_title(track.name.clone());
    tag.set_artist(
        track
            .artists
            .iter()
            .map(|artist| artist.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    );
    tag.set_album(track.album.name.clone());
    let album = spotify_rs::album(track.album.id.clone())
        .get(&spotify)
        .await?;
 
    if !album.genres.is_empty() {
        tag.set_genre(
            album
                .genres
                .iter()
                .map(|genre| genre.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
    } else {
        let song_artist = spotify_rs::get_artist(&album.artists[0].id, &spotify).await?;
        tag.set_genre(
            song_artist
                .genres
                .iter()
                .map(|genre| genre.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }

    tag.set_disk(track.disc_number);

    let image_url = &album.images[0].url;
    let image_bytes = reqwest::get(image_url).await?.bytes().await?.to_vec();

    let mime_type = detect_image_mime_type(&image_bytes);

    let front_cover = Picture::new_unchecked(
        PictureType::CoverFront,
        Some(mime_type),
        Some("Cover".to_string()),
        image_bytes,
    );
    tag.push_picture(front_cover);
    tag.set_track(track.track_number);
    tag.set_track_total(album.total_tracks);
    tag.set_year(album.release_date[..4].parse::<u32>().unwrap_or(0));
        
    let write_options = WriteOptions::new()
        .use_id3v23(true)
        .remove_others(false)
        .respect_read_only(false);

    tagged_file
        .save_to_path(path.clone(), write_options)
        .expect("ERROR: Failed to write the tag!");

    Ok(())
}
