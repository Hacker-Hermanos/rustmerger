// Import required dependencies
use clap::{Parser, Subcommand}; // For command-line argument parsing
use std::path::PathBuf;         // For handling file paths
use log::LevelFilter;           // For controlling log levels

// Main CLI structure that defines the application's command-line interface
#[derive(Parser)]
#[command(
    name = "rustmerger",
    about = "Fast parallel merging and deduplication of wordlists and rules",
    version,
    author,
    long_about = None
)]
pub struct Cli {
    // Global verbose flag that can be used multiple times (-v, -vv, etc.)
    // Each occurrence increases the verbosity level
    #[arg(
        global = true,        // Available to all subcommands
        short = 'v',          // Can be used as -v
        long = "verbose",     // Can be used as --verbose
        action = clap::ArgAction::Count,  // Counts number of occurrences
        help = "Set verbosity level (-v: debug, -vv: trace)"
    )]
    verbose: u8,

    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, default_value = "info")]
    log_level: String,
}

// Enum defining all available subcommands
#[derive(Subcommand)]
pub enum Commands {
    // Merge subcommand for combining wordlists and rules
    #[command(about = "Merge wordlists and rules")]
    Merge(MergeArgs),

    // Generate configuration file subcommand
    #[command(about = "Generate configuration file")]
    GenerateConfig(GenerateConfigArgs),

    // Interactive setup subcommand
    #[command(about = "Run guided setup")]
    GuidedSetup(GuidedSetupArgs),

    // Resume interrupted operations subcommand
    #[command(about = "Resume interrupted operation")]
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
    #[arg(
        short = 'd',
        long = "debug",
        help = "Enable detailed progress output"
    )]
    pub debug: bool,
}

// Arguments for the generate-config command
#[derive(Parser, Clone)]
pub struct GenerateConfigArgs {
    // Output path for the configuration file
    #[arg(
        help = "Destination path for configuration file",
        value_name = "FILE"
    )]
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
    #[arg(
        help = "Path to saved progress state file",
        value_name = "FILE"
    )]
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