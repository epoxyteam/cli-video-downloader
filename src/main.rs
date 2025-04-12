use clap::Parser;
use video_dl::{Result, commands::Cli, commands::Commands, Config};
use video_dl::utils::dependency_check;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    // Load configuration
    let config = Config::load();
    
    // Check for required dependencies
    let status = dependency_check::check_dependencies(&config).await;
    
    if !dependency_check::minimum_dependencies_available(&status) {
        dependency_check::print_dependency_status(&status);
        return Err(video_dl::Error::CommandExecution { 
            command: "dependency check".to_string(), 
            reason: "Required dependencies not found".to_string() 
        });
    }

    match cli.command {
        Commands::Download { url, output, quality, format } => {
            // For download, we ideally want both yt-dlp and ffmpeg
            if !dependency_check::all_dependencies_available(&status) {
                eprintln!("Warning: Some dependencies are missing. Limited functionality available.");
                dependency_check::print_dependency_status(&status);
                eprintln!("Continuing anyway...");
            }
            
            video_dl::commands::download_command(url, output, quality, format).await
        }
        Commands::Info { url } => {
            video_dl::commands::info_command(url).await
        }
        Commands::Config { action } => {
            video_dl::commands::config_command(action).await
        }
        Commands::Batch { url, file, output_dir, quality, format, parallel } => {
            // For batch download, we ideally want both yt-dlp and ffmpeg
            if !dependency_check::all_dependencies_available(&status) {
                eprintln!("Warning: Some dependencies are missing. Limited functionality available.");
                dependency_check::print_dependency_status(&status);
                eprintln!("Continuing anyway...");
            }
            
            video_dl::commands::batch_download_command(url, file, output_dir, quality, format, parallel).await
        }
    }
}
