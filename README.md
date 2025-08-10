# RustifyDL

A fast, no-fuss Spotify → YouTube music downloader with clean tags.

## Why RustifyDL?
Turn any Spotify track/album/playlist URL into properly tagged audio files. RustifyDL pairs Spotify metadata with audio from YouTube, then writes tidy tags and artwork so your library looks right everywhere.

## Features
- ⚡ Concurrent downloads for speed
- 🏷️ Accurate tags: artist, album, track/disc, genre, year, cover art
- 💾 ID3v2.3 compatible tagging 
- 🔇 Clean, minimal logging (tune with verbosity)
- 🧰 FFmpeg-based conversion (choose bitrate/format)

## Build from source
Prerequisites:
- Rust (stable)
- FFmpeg on PATH

Build a release binary:
```bash
cargo build --release
```

## Install to PATH
Choose one approach.

- Option A: cargo install from the local repo
```bash
cargo install --path . --force
```
This will place the binary in Cargo’s bin directory (e.g., Windows: %USERPROFILE%\.cargo\bin, Linux/macOS: ~/.cargo/bin). Ensure that directory is on your PATH.

## Update
Pull latest changes and reinstall:
```bash
git pull
cargo install --path . --force
```

## Usage
Common options (see `--help` for full list):
- `-o, --output-dir <PATH>`  Output folder (default: `./output`)
- `--concurrent-downloads <N>`  Parallel downloads (e.g., 6 or 10)
- `--bitrate <RATE>`  FFmpeg bitrate, e.g., `192k`, `256k`, `320k`
- `--format <EXT>`  Output format, e.g., `mp3`, `m4a`, `opus`, `flac`
- `-v, --verbosity <LEVEL>`  `none`, `info`, `debug`, `full`
- `--no-dupes`  Skip duplicate track names when collecting

Example:
```bash
rustifydl "https://open.spotify.com/album/..." -v info --format mp3 --bitrate 192k --concurrent-downloads 8
```

## Configuration (Automatic)
RustifyDL manages Spotify API credentials automatically. On first use it creates a config file and reuses it next time—no need to pass credentials on the command line.

Config location examples:
- Windows: `%APPDATA%/RustifyDL/config.toml`
- Linux:   `~/.config/RustifyDL/config.toml`
- macOS:   `~/Library/Application Support/RustifyDL/config.toml`

To reset, delete the file and run again.

## Notes
- Tags are written with ID3v2.3 for maximum compatibility.
- Keep FFmpeg updated for best results across formats.

## Project Structure
```
src/
├── lib.rs         # Library API & orchestration
├── metadata.rs    # Tag writing (lofty)
├── spotify.rs     # Spotify fetch (spotify-rs)
└── youtube.rs     # YouTube download (rustypipe + ffmpeg)
```

## Status
🚧 Active development

## License
This project is licensed under the terms specified in the [LICENSE](LICENSE) file.

## Support
If you encounter issues or have questions, please [open an issue](https://github.com/ThePangel/RustifyDL/issues).

---
Built with Rust 🦀 and 💖 by thepangel
