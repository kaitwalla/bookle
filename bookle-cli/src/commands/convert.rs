//! Convert command implementation

use anyhow::{Context, Result};
use bookle_core::decoder::decoder_for_extension;
use bookle_core::encoder::encoder_for_format;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

/// Convert an ebook from one format to another
pub fn convert(input: &str, output: &str, format: &str) -> Result<()> {
    let input_path = Path::new(input);
    let output_path = Path::new(output);

    // Get file extension
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .context("Could not determine input file extension")?;

    // Get decoder
    let decoder = decoder_for_extension(ext)
        .with_context(|| format!("No decoder available for .{} files", ext))?;

    // Get encoder
    let encoder = encoder_for_format(format)
        .with_context(|| format!("No encoder available for {} format", format))?;

    // Set up progress bar with animation
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    // Read input file
    pb.set_message("Reading input file...");
    let file = File::open(input_path)
        .with_context(|| format!("Failed to open input file: {}", input))?;
    let mut reader = BufReader::new(file);

    // Decode
    pb.set_message("Decoding...");
    let book = decoder
        .decode(&mut reader)
        .with_context(|| format!("Failed to decode {}", input))?;

    tracing::info!(
        "Decoded '{}' with {} chapters",
        book.metadata.title,
        book.chapters.len()
    );

    // Encode
    pb.set_message(format!("Encoding to {}...", encoder.format_name()));
    let mut output_file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output))?;

    encoder
        .encode(&book, &mut output_file)
        .with_context(|| format!("Failed to encode to {}", format))?;

    pb.finish_with_message(format!(
        "Converted '{}' to {} -> {}",
        book.metadata.title,
        encoder.format_name(),
        output
    ));

    Ok(())
}
