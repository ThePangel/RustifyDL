use clap::Parser;
use dirs;
use log::{error, info};
use regex::Regex;
use rustifydl::{download_spotify, DownloadOptions};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use tokio;

#[derive(Deserialize, Serialize)]
struct Config {
    client_id: String,
    client_secret: String,
}

#[derive(Parser, Clone)]
pub struct Cli {
    pub url: String,

    #[arg(long = "client-id")]
    pub client_id: Option<String>,

    #[arg(long = "client-secret")]
    pub client_secret: Option<String>,

    #[arg(long = "output-dir", short, default_value = "./output")]
    pub output_dir: String,

    #[arg(long = "concurrent-downloads", short, default_value_t = 15)]
    pub concurrent_downloads: usize,

    #[arg(long = "no-dupes", action = clap::ArgAction::SetTrue)]
    pub no_dupes: bool,

    #[arg(
        long = "bitrate",
        short,
        default_value = "192k",
        value_parser = clap::builder::PossibleValuesParser::new([
            "8k", "16k", "24k", "32k", "40k", "48k", "64k", "80k", "96k", "112k", "128k", "160k", "192k", "224k", "256k", "320k"
        ])
    )]
    pub bitrate: String,

    #[arg(
        long = "format",
        short,
        default_value = "mp3",
        value_parser = clap::builder::PossibleValuesParser::new([
            "mp3", "flac", "ogg", "opus", "m4a", "wav"
        ])
    )]
    pub format: String,

    #[arg(
        long = "verbosity",
        short,
        default_value = "info",
        value_parser = clap::builder::PossibleValuesParser::new([
            "info", "debug", "error", "none", "full"
        ])
    )]
    pub verbosity: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Cli::parse();
    let (client_id, client_secret) =
        if let (Some(id), Some(secret)) = (args.client_id, args.client_secret) {
            (id, secret)
        } else {
            let config = check_api_keys().await?;
            (config.client_id, config.client_secret)
        };

    let options = DownloadOptions {
        url: args.url,
        client_id,
        client_secret,
        output_dir: args.output_dir,
        concurrent_downloads: args.concurrent_downloads,
        no_dupes: args.no_dupes,
        bitrate: args.bitrate,
        format: args.format,
        verbosity: args.verbosity
    };
    download_spotify(options).await?;
    Ok(())
}

async fn check_api_keys() -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    let config_dir = dirs::config_dir().ok_or("Could not find a valid config directory.")?;

    let app_config_dir = config_dir.join("RustifyDL");
    fs::create_dir_all(&app_config_dir)?;

    let config_path = app_config_dir.join("config.toml");
    let mut client_id = String::new();
    let mut client_secret = String::new();
    if config_path.exists() && config_path.is_file() && fs::metadata(&config_path)?.len() != 0 {
        let content = fs::read_to_string(&config_path)?;
        let keys;
        match toml::from_str::<Config>(&content) {
            Ok(parsed_keys) => keys = parsed_keys,
            Err(e) => {
                error!("Malformed config file: {}", e);
                fs::remove_file(&config_path)?;
                info!("The malformed config file has been deleted. Please re-enter your keys.");
                loop {
                    print!("Enter Client ID: ");
                    std::io::stdout().flush()?;
                    client_id.clear();
                    std::io::stdin().read_line(&mut client_id)?;
                    if verify_key(client_id.trim()) {
                        break;
                    }
                    println!("Invalid Client ID. It must be a 32-character hexadecimal string.");
                }
                loop {
                    print!("Enter Client Secret: ");
                    std::io::stdout().flush()?;
                    client_secret.clear();
                    std::io::stdin().read_line(&mut client_secret)?;
                    if verify_key(client_secret.trim()) {
                        break;
                    }
                    println!(
                        "Invalid Client Secret. It must be a 32-character hexadecimal string."
                    );
                }
                let keys = Config {
                    client_id: client_id.trim().to_string(),
                    client_secret: client_secret.trim().to_string(),
                };

                let value = toml::to_string(&keys)?;
                fs::write(&config_path, value)?;
                println!("Configuration saved to: {}", config_path.display());

                return Ok(keys);
            }
        };

        if keys.client_id.trim().is_empty() {
            eprint!("Client id is empty or missing!");
            loop {
                print!("Enter Client Secret: ");
                std::io::stdout().flush()?;
                client_secret.clear();
                std::io::stdin().read_line(&mut client_secret)?;
                if verify_key(client_secret.trim()) {
                    break;
                }
                println!("Invalid Client ID. It must be a 32-character hexadecimal string.");
                let fixed_keys = Config {
                    client_id: client_id.clone(),
                    client_secret: keys.client_secret.clone(),
                };
                let value = toml::to_string(&fixed_keys)?;
                fs::write(&config_path, value)?;
            }
        }
        if keys.client_secret.trim().is_empty() {
            eprint!("Client id is empty or missing!");
            loop {
                print!("Enter Client ID: ");
                std::io::stdout().flush()?;
                client_id.clear();
                std::io::stdin().read_line(&mut client_id)?;
                if verify_key(client_id.trim()) {
                    break;
                }
                println!("Invalid Client ID. It must be a 32-character hexadecimal string.");
                let fixed_keys = Config {
                    client_id: keys.client_id.clone(),
                    client_secret: client_secret.clone(),
                };
                let value = toml::to_string(&fixed_keys)?;
                fs::write(&config_path, value)?;
            }
        }

        if verify_key(&keys.client_id.trim()) && verify_key(&&keys.client_secret.trim()) {
            return Ok(keys);
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Keys are not valid! Check the config file!",
            )));
        }
    } else {
        println!(
            "No config file found or keys are empty, please enter the Spotify API keys:\n If you don't have them, here is a guide: https://developer.spotify.com/documentation/web-api"
        );
        loop {
            print!("Enter Client ID: ");
            std::io::stdout().flush()?;
            client_id.clear();
            std::io::stdin().read_line(&mut client_id)?;
            if verify_key(client_id.trim()) {
                break;
            }
            println!("Invalid Client ID. It must be a 32-character hexadecimal string.");
        }
        loop {
            print!("Enter Client Secret: ");
            std::io::stdout().flush()?;
            client_secret.clear();
            std::io::stdin().read_line(&mut client_secret)?;
            if verify_key(client_secret.trim()) {
                break;
            }
            println!("Invalid Client Secret. It must be a 32-character hexadecimal string.");
        }
        let keys = Config {
            client_id: client_id.trim().to_string(),
            client_secret: client_secret.trim().to_string(),
        };

        let value = toml::to_string(&keys)?;
        fs::write(&config_path, value)?;
        println!("Configuration saved to: {}", config_path.display());

        return Ok(keys);
    }
}

fn verify_key(key: &str) -> bool {
    let re = Regex::new(r"^[[:xdigit:]]{32}$").unwrap();
    re.is_match(key)
}
