use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rustypipe::client::RustyPipe;
use rustypipe::param::StreamFilter;
use rustypipe_downloader::DownloaderBuilder;
use std::fs;
use std::process::Command;

pub(crate) async fn search_yt(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let rp = RustyPipe::new();
    let search_results = rp.query().music_search_tracks(name).await?;

    download(search_results.items.items[0].id.as_str(), name).await?;
    Ok(())
}

async fn download(id: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let multi = MultiProgress::new();
    let style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {percent}% {bytes}/{total_bytes} {msg}")?
        .progress_chars("█▉▊▋▌▍▎▏ ");

    let dl = DownloaderBuilder::new()
        .multi_progress(multi.clone())
        .progress_style(style.clone())
        .build();

    let pb = multi.add(ProgressBar::new(0));
    let filter_audio = StreamFilter::new().no_video();

    dl.id(id)
        .stream_filter(filter_audio)
        .to_file(format!("./output/{}.opus", name))
        .progress_bar(pb.clone())
        .download()
        .await?;

    convert_to_mp3(
        format!("./output/{}.opus", name).as_str(),
        format!("./output/{}.mp3", name).as_str(),
    )?;
    Ok(())
}

fn convert_to_mp3(input_file: &str, output_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("ffmpeg")
        .args([
            "-i",
            input_file,
            "-c:a",
            "mp3",
            "-b:a",
            "320k",
            "-y",
            output_file,
        ])
        .output()?;

    if !output.status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "FFmpeg conversion failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        )));
    }
    fs::remove_file(input_file)?;

    Ok(())
}
