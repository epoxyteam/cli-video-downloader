# Video Downloader CLI

A fast and user-friendly command-line tool for downloading videos from various platforms written in Rust.

## Supported Platforms

- YouTube (basic support)
- More platforms coming soon:
  - TikTok
  - Douyin
  - Reddit
  - Instagram

## Features

- Fast, concurrent downloads
- Progress tracking
- Quality selection
- Format selection
- Platform auto-detection
- Clean command-line interface

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/video-dl.git
cd video-dl

# Build the project
cargo build --release

# The binary will be available in target/release/video-dl
```

## Usage

### Download a Video

```bash
# Basic usage (auto-selects best quality)
video-dl download --url "https://www.youtube.com/watch?v=..."

# Specify quality and format
video-dl download --url "https://www.youtube.com/watch?v=..." --quality 1080p --format mp4

# Specify output location
video-dl download --url "https://www.youtube.com/watch?v=..." --output "my-video.mp4"
```

### Get Video Information

```bash
video-dl info --url "https://www.youtube.com/watch?v=..."
```

### Supported Quality Options

- `low`
- `medium`
- `high`
- `720p`
- `1080p`
- `2160p` (4K)
- Custom resolutions (e.g., "1440p")

### Supported Formats

- `mp4`
- `webm`
- `mov`
- Others depending on platform support

## Development

### Project Structure

```
src/
├── main.rs           # CLI entry point
├── lib.rs           # Core types and traits
├── error.rs         # Error types
├── downloader.rs    # Download management
└── platform/        # Platform implementations
    ├── mod.rs       # Platform trait definitions
    ├── detector.rs  # Platform detection
    ├── youtube.rs   # YouTube implementation
    └── ...         # Other platforms
```

### Adding New Platforms

To add support for a new platform:

1. Create a new file in `src/platform/` for your platform
2. Implement the `Platform` trait
3. Register the platform in `src/platform/mod.rs`

Example platform implementation:

```rust
use async_trait::async_trait;
use url::Url;

#[async_trait]
impl Platform for MyPlatform {
    fn name(&self) -> &'static str {
        "MyPlatform"
    }

    fn supports_url(&self, url: &Url) -> bool {
        url.host_str()
            .map(|host| host.ends_with("myplatform.com"))
            .unwrap_or(false)
    }

    async fn extract_info(&self, url: &Url) -> Result<VideoInfo> {
        // Implementation
    }

    async fn download_video(
        &self,
        info: &VideoInfo,
        format_id: &str,
        output_path: &Path,
        progress_tx: Arc<watch::Sender<f64>>,
    ) -> Result<()> {
        // Implementation
    }
}
```

## License

Apache-2.0 License. See [LICENSE](LICENSE) for details.