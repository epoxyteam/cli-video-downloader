use clap::Parser;
use video_dl::{Result, commands::Cli, commands::Commands};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Download { url, output, quality, format } => {
            video_dl::commands::download_command(url, output, quality, format).await
        }
        Commands::Info { url } => {
            video_dl::commands::info_command(url).await
        }
    }
}
