//! Bookle CLI - Command-line interface for ebook management

mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Parse and validate jobs argument (must be at least 1)
fn parse_jobs(s: &str) -> Result<usize, String> {
    let n: usize = s.parse().map_err(|_| format!("'{}' is not a valid number", s))?;
    if n < 1 {
        Err("jobs must be at least 1".to_string())
    } else {
        Ok(n)
    }
}

#[derive(Parser)]
#[command(name = "bookle")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert an ebook to another format
    Convert {
        /// Input file path
        input: String,

        /// Output file path
        #[arg(short, long)]
        output: String,

        /// Output format (epub, pdf)
        #[arg(short, long, default_value = "epub")]
        format: String,
    },

    /// Display information about an ebook
    Info {
        /// Input file path
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Validate an ebook file
    Validate {
        /// Input file path
        input: String,

        /// Strict validation mode
        #[arg(long)]
        strict: bool,
    },

    /// Batch convert multiple ebooks
    Batch {
        /// Input directory
        input_dir: String,

        /// Output directory
        #[arg(short, long)]
        output_dir: String,

        /// Output format (epub, pdf)
        #[arg(short, long, default_value = "epub")]
        format: String,

        /// Number of parallel jobs (must be at least 1)
        #[arg(short, long, default_value = "4", value_parser = parse_jobs)]
        jobs: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        "bookle_cli=debug,bookle_core=debug"
    } else {
        "bookle_cli=info"
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(filter))
        .with(tracing_subscriber::fmt::layer())
        .init();

    match cli.command {
        Commands::Convert {
            input,
            output,
            format,
        } => commands::convert(&input, &output, &format),

        Commands::Info { input, json } => commands::info(&input, json),

        Commands::Validate { input, strict } => commands::validate(&input, strict),

        Commands::Batch {
            input_dir,
            output_dir,
            format,
            jobs,
        } => commands::batch(&input_dir, &output_dir, &format, jobs),
    }
}
