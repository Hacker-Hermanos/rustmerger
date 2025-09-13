// ============================================================================
// CLI Module - Command-Line Interface Definitions
//
// This module defines the complete command-line interface for rustmerger using
// the clap crate with derive macros. It provides a comprehensive CLI with
// multiple subcommands for different operations.
//
// Design Principles:
// - Clear command structure with specific subcommands
// - Comprehensive help text with examples
// - Memory usage warnings for large operations
// - Flexible configuration options
// ============================================================================

use clap::{Parser, Subcommand}; // Modern command-line parsing with derive macros
use log::LevelFilter;
use std::path::PathBuf; // Cross-platform file path handling // Logging level configuration

// ============================================================================
// MAIN CLI STRUCTURE
// ============================================================================

/// rustmerger - High-Performance Wordlist & Rule Merger
///
/// A specialized tool for password cracking workflows that efficiently merges
/// and deduplicates wordlists and hashcat rule files. Optimized for large-scale
/// operations with parallel processing and resume capabilities.
///
/// Key Features:
/// - Parallel processing with configurable thread counts
/// - Memory-efficient handling of multi-GB files
/// - HashSet-based deduplication for optimal performance
/// - Resume capability for interrupted operations
/// - Comprehensive hashcat compatibility
///
/// Examples:
///   rustmerger merge -w wordlists.txt --output-wordlist merged.txt
///   rustmerger generate-config template.json --template
///   rustmerger guided-setup interactive.json
///   rustmerger resume checkpoint.json
///
/// Performance Tips:
/// - Use SSD storage for better I/O performance
/// - Monitor memory usage with large files (estimate: unique_lines * 100 bytes)
/// - Adjust thread count based on CPU cores and available memory
///
/// Security Notice:
/// This tool is designed for authorized security testing only.
/// Always ensure proper authorization before testing password systems.
#[derive(Parser)]
#[command(
    name = "rustmerger",
    about = "High-performance wordlist and hashcat rule merger for password cracking workflows",
    long_about = "rustmerger efficiently merges and deduplicates wordlists and hashcat rule files using parallel processing. \
                  Designed for password security testing and penetration testing operations with support for multi-GB files, \
                  resume functionality, and comprehensive error handling.",
    version,
    author = "Robert Pimentel (@pr0b3r7) <https://github.com/pr0b3r7>",
    after_help = "Examples:\n  \
                  rustmerger merge -w input_files.txt --output-wordlist merged.txt\n  \
                  rustmerger merge -r rules.txt --output-rules combined.rule\n  \
                  rustmerger generate-config config.json --template\n  \
                  rustmerger guided-setup interactive_config.json\n  \
                  rustmerger resume operation_checkpoint.json\n\n\
                  For detailed usage examples and performance tuning:\n  \
                  https://github.com/Hacker-Hermanos/rustmerger#usage"
)]
pub struct Cli {
    /// Increase verbosity level (can be used multiple times)
    ///
    /// Controls the amount of debug information displayed:
    /// - (none): INFO level - basic operation messages
    /// - -v: DEBUG level - detailed processing information  
    /// - -vv: TRACE level - comprehensive debugging output
    ///
    /// Use higher verbosity levels when troubleshooting performance
    /// issues or investigating processing errors.
    #[arg(
        global = true,        // Available to all subcommands
        short = 'v',          // Short flag: -v
        long = "verbose",     // Long flag: --verbose
        action = clap::ArgAction::Count,  // Counts occurrences: -vv = 2
        help = "Increase verbosity (-v: debug, -vv: trace)",
        long_help = "Set verbosity level for debugging and monitoring:\n\
                     (no flag): INFO level - basic operation status\n\
                     -v: DEBUG level - detailed processing information\n\
                     -vv: TRACE level - comprehensive debugging output\n\n\
                     Higher verbosity levels are useful for:\n\
                     - Troubleshooting performance issues\n\
                     - Investigating file processing errors\n\
                     - Monitoring memory usage patterns\n\
                     - Understanding deduplication behavior"
    )]
    verbose: u8,

    /// The operation to perform
    #[command(subcommand)]
    pub command: Commands,

    /// Set the logging level explicitly
    ///
    /// Alternative to verbose flags for precise log level control.
    /// Accepts: error, warn, info, debug, trace
    #[arg(
        long,
        default_value = "info",
        help = "Set log level explicitly [error|warn|info|debug|trace]",
        long_help = "Set the logging level explicitly instead of using verbose flags.\n\
                     Available levels (in order of verbosity):\n\
                     - error: Only critical errors that prevent operation\n\
                     - warn: Warnings about potential issues or suboptimal conditions\n\
                     - info: General information about operation progress (default)\n\
                     - debug: Detailed information useful for troubleshooting\n\
                     - trace: Extremely detailed information for deep debugging\n\n\
                     Note: This overrides any -v flags if specified."
    )]
    log_level: String,
}

// ============================================================================
// SUBCOMMAND DEFINITIONS
// ============================================================================

/// Available subcommands for different operations
#[derive(Subcommand)]
pub enum Commands {
    /// Merge and deduplicate wordlists or rule files
    ///
    /// This is the primary operation for combining multiple wordlist or rule files
    /// into a single, deduplicated output file. Uses parallel processing and
    /// memory-efficient algorithms for optimal performance.
    ///
    /// Examples:
    ///   rustmerger merge -w wordlists.txt --output-wordlist merged.txt
    ///   rustmerger merge -r rules.txt --output-rules combined.rule
    ///   rustmerger merge -c config.json --debug
    #[command(
        about = "Merge and deduplicate wordlists or hashcat rule files",
        long_about = "Merge multiple wordlist or rule files into a single deduplicated output. \
                      Uses HashSet-based deduplication with parallel processing for optimal performance. \
                      Supports resume functionality and comprehensive error handling.\n\n\
                      Memory Usage: Approximately unique_lines * 100 bytes\n\
                      Performance: Scales with number of unique entries, not total file size\n\n\
                      Examples:\n  \
                      rustmerger merge -w input_files.txt --output-wordlist merged.txt\n  \
                      rustmerger merge -r rule_files.txt --output-rules combined.rule\n  \
                      rustmerger merge -c config.json --progress-file checkpoint.json"
    )]
    Merge(MergeArgs),

    /// Generate a configuration file template
    ///
    /// Creates a JSON configuration file with default settings that can be
    /// customized for repeated operations. Useful for batch processing or
    /// when using the same settings across multiple operations.
    ///
    /// Example:
    ///   rustmerger generate-config my_config.json --template
    #[command(
        about = "Generate a JSON configuration file template",
        long_about = "Generate a JSON configuration file with default settings for reuse. \
                      Configuration files allow you to specify all operation parameters \
                      in a single file, making it easy to repeat operations or share \
                      settings across a team.\n\n\
                      Generated template includes:\n\
                      - Input and output file paths\n\
                      - Thread count configuration\n\
                      - Logging preferences\n\
                      - Debug mode settings\n\n\
                      Example:\n  \
                      rustmerger generate-config template.json --template"
    )]
    GenerateConfig(GenerateConfigArgs),

    /// Run interactive guided setup
    ///
    /// Step-by-step configuration creation with interactive prompts.
    /// Ideal for first-time users or when setting up complex configurations.
    ///
    /// Example:
    ///   rustmerger guided-setup interactive_config.json
    #[command(
        about = "Run interactive guided setup for configuration",
        long_about = "Interactive configuration creation with step-by-step prompts. \
                      This mode guides you through all available options with \
                      explanations and default values.\n\n\
                      The guided setup will prompt for:\n\
                      - Input file paths and formats\n\
                      - Output file locations\n\
                      - Performance settings (thread count)\n\
                      - Logging and debug preferences\n\n\
                      Perfect for:\n\
                      - First-time users learning the tool\n\
                      - Complex configuration setups\n\
                      - Ensuring all options are properly configured\n\n\
                      Example:\n  \
                      rustmerger guided-setup my_operation_config.json"
    )]
    GuidedSetup(GuidedSetupArgs),

    /// Resume an interrupted operation from checkpoint
    ///
    /// Continue processing from a saved checkpoint file when an operation
    /// was interrupted (Ctrl+C, system shutdown, etc.). Preserves all
    /// progress and continues from the last processed file.
    ///
    /// Example:
    ///   rustmerger resume operation_checkpoint.json
    #[command(
        about = "Resume an interrupted operation from checkpoint file",
        long_about = "Resume processing from a previously saved checkpoint file. \
                      When operations are interrupted (Ctrl+C, system shutdown, power loss), \
                      rustmerger automatically saves progress to a checkpoint file.\n\n\
                      Resume functionality preserves:\n\
                      - List of already processed files\n\
                      - Current position in processing queue\n\
                      - All configuration settings\n\
                      - Partial deduplication state\n\n\
                      Checkpoints are automatically created when:\n\
                      - Using --progress-file flag during merge operations\n\
                      - Graceful shutdown via Ctrl+C\n\
                      - Encountering recoverable errors\n\n\
                      Example:\n  \
                      rustmerger resume wordlist_operation_checkpoint.json"
    )]
    Resume(ResumeArgs),
}

// Structure defining all possible arguments for the merge command
#[derive(Parser, Clone)]
pub struct MergeArgs {
    // Input file containing list of wordlist paths
    #[arg(
        short = 'w',
        long = "wordlists-file",
        help = "Text file containing one wordlist path per line",
        value_name = "FILE"
    )]
    pub wordlists_file: Option<PathBuf>,

    // Input file containing list of rule paths
    #[arg(
        short = 'r',
        long = "rules-file",
        help = "Text file containing one rule path per line",
        value_name = "FILE"
    )]
    pub rules_file: Option<PathBuf>,

    // Output path for merged wordlist
    #[arg(
        long = "output-wordlist",
        help = "Destination path for merged and deduplicated wordlist",
        value_name = "FILE"
    )]
    pub output_wordlist: Option<PathBuf>,

    // Output path for merged rules
    #[arg(
        long = "output-rules",
        help = "Destination path for merged and deduplicated rules",
        value_name = "FILE"
    )]
    pub output_rules: Option<PathBuf>,

    // Configuration file path
    #[arg(
        short = 'c',
        long = "config",
        help = "JSON configuration file with default settings",
        value_name = "FILE"
    )]
    pub config: Option<PathBuf>,

    // Progress state file for resume capability
    #[arg(
        long = "progress-file",
        help = "Save progress state for resume capability",
        value_name = "FILE"
    )]
    pub progress_file: Option<PathBuf>,

    // Debug mode flag
    #[arg(short = 'd', long = "debug", help = "Enable detailed progress output")]
    pub debug: bool,
}

// Arguments for the generate-config command
#[derive(Parser, Clone)]
pub struct GenerateConfigArgs {
    // Output path for the configuration file
    #[arg(help = "Destination path for configuration file", value_name = "FILE")]
    pub output: PathBuf,

    // Flag to generate template configuration
    #[arg(
        short = 't',
        long = "template",
        help = "Generate default configuration template"
    )]
    pub template: bool,
}

// Arguments for the guided-setup command
#[derive(Parser, Clone)]
pub struct GuidedSetupArgs {
    // Output path for the generated configuration
    #[arg(
        help = "Destination path for interactive configuration",
        value_name = "FILE"
    )]
    pub output: PathBuf,
}

// Arguments for the resume command
#[derive(Parser, Clone)]
pub struct ResumeArgs {
    // Path to the progress state file
    #[arg(help = "Path to saved progress state file", value_name = "FILE")]
    pub progress_file: PathBuf,
}

// Implementation of helper methods for the Cli struct
impl Cli {
    // Convert verbose flag count to appropriate log level
    pub fn log_level(&self) -> LevelFilter {
        match self.log_level.as_str() {
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        }
    }

    // Add this new method
    pub fn verbose_count(&self) -> u8 {
        self.verbose
    }
}
