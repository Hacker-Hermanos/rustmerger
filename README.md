# File Merger Tool

## Overview

A robust command-line tool built in Rust that makes merging and deduplicating text files a breeze. Whether you're dealing with small files or massive datasets, this tool handles the heavy lifting with parallel processing and smart error handling.

## Key Features

### Core Functionality

- **Smart File Merging**: Feed it a list of file paths via `-i/--input-files`, and it'll combine them into a single output file (`-o/--output-files`).
- **No More Duplicates**: Uses a `HashSet` under the hood to ensure each line appears exactly once in your final output.
- **Memory-Friendly**: Processes files in 10MB chunks by default, so your RAM stays happy.
- **Optimized I/O**: Uses generous buffer sizes (32MB read, 16MB write) to keep things moving quickly.

### Performance Features

- **Parallel Processing**: Spreads the work across 10 threads by default (but you can adjust this).
- **Resource-Conscious**: Chunks files to keep memory usage in check, even with large files.
- **Know What's Happening**: Shows you exactly where you are with progress bars for:
  - Overall progress
  - Current file
  - Deduplication status
- **Your Tool, Your Rules**: Tweak buffer sizes and other settings to match your needs.

### Error Handling & Reliability

- **Keeps Going**: Logs errors without stopping, because one bad file shouldn't ruin everything.
- **UTF-8 Problems? No Problem**: Skips problematic lines and keeps moving.
- **Checks First**: Makes sure all your input files exist and are readable before starting.
- **Safe Writes**: Uses atomic writing to protect your output file from corruption.

### Resume Capability

- **Never Lose Progress**: Creates checkpoint files as it works.
- **Ctrl+C Friendly**: Saves its state when interrupted so you can pick up where you left off.
- **Easy Resumption**: Just use `--resume <progress-file>` to continue an interrupted job.
- **Knows Its Place**: Keeps track of exactly where it stopped, down to the line.

## Author

Robert Pimentel

- GitHub: [@pr0b3r7](https://github.com/pr0b3r7)
- LinkedIn: [pimentelrobert1](https://linkedin.com/in/pimentelrobert1)
- Website: [hackerhermanos.com](https://www.hackerhermanos.com)

## Dependencies

This project relies on several high-quality Rust crates to provide its functionality:

### Core Dependencies

- **tokio** (1.36) - Asynchronous runtime powering parallel processing
- **clap** (4.4) - Command-line argument parsing
- **serde** (1.0) - Serialization framework for configuration
- **anyhow** (1.0.91) - Error handling with context

### File Processing

- **async-compression** (0.4.17) - Handles various compression formats (bzip2, gzip, xz)
- **zip** (2.2.0) - ZIP archive support
- **unrar** (0.5.6) - RAR archive support
- **sevenz-rust** (0.6.1) - 7z archive support
- **tar** (0.4.42) - TAR archive support

### User Interface

- **indicatif** (0.17) - Progress bars and spinners
- **dialoguer** (0.11.0) - Interactive command prompts
- **crossterm** (0.28.1) - Terminal manipulation
- **terminal_size** (0.4.0) - Terminal dimensions detection

### Utilities

- **chrono** (0.4.38) - Date and time handling
- **uuid** (1.11.0) - Unique identifier generation
- **sha2** (0.10.8) - Cryptographic hashing
- **encoding_rs** (0.8.35) - Character encoding support
- **sys-info** (0.9.1) - System information gathering

### Networking

- **reqwest** (0.12.9) - HTTP client with streaming support
- **url** (2.5.2) - URL parsing and manipulation

### Logging and Error Handling

- **env_logger** (0.11.5) - Environment-based logging
- **log** (0.4.22) - Logging framework
- **thiserror** (1.0.65) - Custom error types

### Signal Handling

- **ctrlc** (3.4.5) - Ctrl+C signal handling
- **signal-hook** (0.3.17) - OS signal handling

## Installation

### You'll Need

- Rust toolchain (1.70+)
- Cargo package manager

### Getting Started

1. Grab the code:
   ```sh
   git clone https://github.com/yourusername/file-merger-tool.git
   cd file-merger-tool
   ```

2. Build it:
   ```sh
   cargo build --release
   ```

3. Want it system-wide? (Optional):
   ```sh
   sudo cp target/release/file-merger-tool /usr/local/bin/
   ```

## Usage

### Quick Start

```sh
file-merger-tool merge -w input_list.txt -o merged_output.txt
```

### Command Reference

```
Usage: rustmerger [OPTIONS] <COMMAND>

Commands:
  merge            Merge wordlists and rules
  generate-config  Generate configuration file
  guided-setup     Run guided setup
  resume           Resume interrupted operation
  help             Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...             Set verbosity level (-v: debug, -vv: trace)
      --log-level <LOG_LEVEL>  [default: info]
  -h, --help                   Print help
  -V, --version                Print version
```

#### Merge Command

```
Usage: rustmerger merge [OPTIONS]

Options:
  -v, --verbose...              Set verbosity level (-v: debug, -vv: trace)
  -w, --wordlists-file <FILE>   Text file containing one wordlist path per line
  -r, --rules-file <FILE>       Text file containing one rule path per line
      --output-wordlist <FILE>  Destination path for merged and deduplicated wordlist
      --output-rules <FILE>     Destination path for merged and deduplicated rules
  -c, --config <FILE>           JSON configuration file with default settings
      --progress-file <FILE>    Save progress state for resume capability
  -d, --debug                   Enable detailed progress output
  -h, --help                    Print help
```

#### Generate Config Command

```
Usage: rustmerger generate-config [OPTIONS] <FILE>

Arguments:
  <FILE>  Destination path for configuration file

Options:
  -t, --template    Generate default configuration template
  -v, --verbose...  Set verbosity level (-v: debug, -vv: trace)
  -h, --help        Print help
```

#### Guided Setup Command

```
Usage: rustmerger guided-setup [OPTIONS] <FILE>

Arguments:
  <FILE>  Destination path for interactive configuration

Options:
  -v, --verbose...  Set verbosity level (-v: debug, -vv: trace)
  -h, --help        Print help
```

#### Sample Configuration File

```json
{
  "input_files": "/tmp/wordlists_to_merge_dev.txt",
  "output_files": "/tmp/merged_wordlist.txt",
  "threads": 90,
  "verbose": true,
  "debug": true
}
```

### Under the Hood

#### How It Works

The heavy lifting happens in the `FileProcessor` struct (`src/processing.rs`). Here's what makes it tick:

1. **Smart File Reading**: 
   - Uses async I/O with `tokio` for non-blocking file access
   - Buffers reads to minimize system calls

2. **Reliable Error Handling**:
   - Logs issues but keeps going
   - Won't let one bad file stop the whole show

3. **Line-by-Line Processing**:
   - Handles each line individually
   - Gracefully skips UTF-8 issues

4. **Progress Tracking**:
   - Keeps tabs on processed files
   - Makes resuming interrupted jobs seamless

#### Performance Tricks

1. **Parallel Power**:
   - Spreads work across multiple threads (default: 10)
   - Built on `tokio` for efficient async processing

2. **Smart Deduplication**:
   - Uses `HashSet` for O(1) lookups
   - Keeps memory usage in check

3. **Visual Feedback**:
   - Real-time progress bars
   - Shows you exactly what's happening

4. **Interruption-Proof**:
   - Handles Ctrl+C gracefully
   - Saves progress for later
   - Managed by `AppState` in `src/app_state.rs`

5. **Flexible Configuration**:
   - JSON config support via `--config <path>`
   - Interactive setup with `--guided-setup`

This tool is built to be reliable, efficient, and adaptable to your needs. Whether you're merging a few files or processing thousands, it's got you covered.