# CLI Video Downloader

A fast and user-friendly command-line video downloader built in Rust. This application allows you to download videos from various platforms (currently YouTube, with more platforms coming soon).

## Features

- Download videos from YouTube (more platforms coming soon)
- Select video quality and format
- Show detailed video information
- Customizable configuration
- Progress bar display
- Support for various output formats

## Installation

### Prerequisites

Make sure you have `yt-dlp` installed and available in your PATH. This tool uses `yt-dlp` under the hood.

- Install yt-dlp: https://github.com/yt-dlp/yt-dlp#installation

### Install from Source

```bash
git clone https://github.com/yourusername/cli-video-downloader.git
cd cli-video-downloader
cargo build --release
```

The binary will be available at `target/release/video-dl`.

## Usage

### Download a Video

```bash
video-dl download -u https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

Specify quality and format:

```bash
video-dl download -u https://www.youtube.com/watch?v=dQw4w9WgXcQ -q 720p -f mp4
```

Specify output path:

```bash
video-dl download -u https://www.youtube.com/watch?v=dQw4w9WgXcQ -o my-video.mp4
```

### Get Video Information

```bash
video-dl info -u https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

### Manage Configuration

Show configuration:

```bash
video-dl config get
```

Get a specific configuration value:

```bash
video-dl config get -k download_dir
```

Set a configuration value:

```bash
video-dl config set -k download_dir -v "/path/to/downloads"
```

Reset configuration to defaults:

```bash
video-dl config reset
```

## Configuration

The application stores configuration in:
- Windows: `%APPDATA%\video-dl\config.toml`
- macOS: `~/Library/Application Support/video-dl/config.toml`
- Linux: `~/.config/video-dl/config.toml`

Available configuration options:

| Key | Description | Default |
|-----|-------------|---------|
| download_dir | Directory where videos are saved | Downloads folder |
| default_quality | Default video quality | "best" |
| default_format | Default video format | "mp4" |
| show_progress | Whether to show progress bars | true |
| overwrite_files | Whether to overwrite existing files | false |
| ytdlp_path | Custom path to yt-dlp executable | None (use PATH) |

## Supported Platforms

- YouTube (including Shorts)

## Contributing

Contributions are welcome! Feel free to open an issue or submit a pull request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.