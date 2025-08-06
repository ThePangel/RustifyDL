use clap::Parser;
use tokio;
use rustify::download_spotify;
#[derive(Parser, Clone)]
pub struct Cli {
    pub url: String,
    pub client_id: String,
    pub client_secret: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Cli::parse();
    download_spotify(&args.url, &args.client_id, &args.client_secret).await?;
    Ok(())
}