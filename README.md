# rustmerger

High-performance wordlist merger for password cracking workflows. Efficiently merges and deduplicates wordlists and rule files with automatic encoding detection.

## Features

- **Fast Processing**: Parallel processing with configurable thread count
- **Smart Deduplication**: Memory-efficient HashSet deduplication
- **Encoding Support**: Automatic detection of UTF-8, Windows-1252, ISO-8859-1
- **Large File Handling**: Processes multi-GB files with streaming
- **Progress Tracking**: Real-time progress bars with resume capability
- **Signal Handling**: Graceful Ctrl+C with progress preservation

## Installation

```bash
git clone https://github.com/Hacker-Hermanos/rustmerger.git
cd rustmerger
cargo build --release
```

## Usage

### Basic Wordlist Merge

```bash
# Create input file list
echo "/path/to/rockyou.txt" > wordlists.txt
echo "/path/to/other.txt" >> wordlists.txt

# Merge and deduplicate
rustmerger merge -w wordlists.txt --output-wordlist merged.txt
```

### Rule File Processing

```bash
echo "/path/to/best64.rule" > rules.txt
echo "/path/to/custom.rule" >> rules.txt

rustmerger merge -r rules.txt --output-rules combined.rule
```

### Configuration File

```bash
# Generate template
rustmerger generate-config config.json

# Use configuration
rustmerger merge -c config.json
```

## Configuration

Create a JSON configuration file:

```json
{
  "input_files": "/path/to/input_list.txt",
  "output_files": "/path/to/output.txt",
  "threads": 10,
  "verbose": true,
  "debug": false
}
```

## Encoding Support

Automatically handles multiple encodings without manual conversion:

- **UTF-8**: Modern wordlists
- **Windows-1252**: rockyou.txt and legacy wordlists
- **ISO-8859-1/15**: European wordlists with special characters

Special characters (é, ñ, ü) are preserved correctly.

## Performance

- **rockyou.txt**: 14.3M lines processed in ~8.7 seconds
- **Memory usage**: Scales with unique entries, not file size
- **Optimal threads**: CPU cores + 2 (default: 10)

## Commands

```bash
rustmerger merge             # Merge wordlists/rules with deduplication
rustmerger generate-config   # Create configuration template
rustmerger guided-setup      # Interactive setup
rustmerger resume           # Resume interrupted operation
```

## Common Issues

### Out of Memory
Reduce thread count in configuration:
```json
{"threads": 4}
```

### Encoding Errors
Enable verbose mode to see encoding detection:
```bash
rustmerger merge -w input.txt --output output.txt --verbose
```

### Performance Issues
- Use SSD storage for large files
- Monitor with `htop` and adjust thread count
- Process very large files in batches if needed

## Dependencies

Key libraries used:
- **tokio**: Async runtime for parallel processing
- **encoding_rs**: Mozilla's encoding library
- **chardetng**: Automatic encoding detection
- **clap**: Command-line interface
- **indicatif**: Progress bars

## Development

```bash
# Run tests
cargo test

# Debug mode
RUST_LOG=debug cargo run -- merge -w input.txt --output output.txt

# Format code
cargo fmt
```

## License

MIT License - see LICENSE file for details.

## Author

Robert Pimentel ([@pr0b3r7](https://github.com/pr0b3r7))