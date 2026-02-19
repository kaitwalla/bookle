//! Validate command implementation

use anyhow::{bail, Context, Result};
use bookle_core::decoder::decoder_for_extension;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Validate an ebook file
pub fn validate(input: &str, _strict: bool) -> Result<()> {
    let input_path = Path::new(input);

    // Get file extension
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .context("Could not determine input file extension")?;

    // Get decoder
    let decoder = decoder_for_extension(ext)
        .with_context(|| format!("No decoder available for .{} files", ext))?;

    // Try to read and decode
    let file =
        File::open(input_path).with_context(|| format!("Failed to open input file: {}", input))?;
    let mut reader = BufReader::new(file);

    match decoder.decode(&mut reader) {
        Ok(book) => {
            println!("Valid {} file", ext.to_uppercase());
            println!("  Title: {}", book.metadata.title);
            println!("  Chapters: {}", book.chapters.len());

            // TODO: Add more validation checks (strict mode)
            // - Check all resource references are valid
            // - Validate TOC structure
            // - Check for empty chapters
            // - Validate metadata completeness

            Ok(())
        }
        Err(e) => {
            eprintln!("Invalid {} file: {}", ext.to_uppercase(), e);
            bail!("Validation failed for {}", input);
        }
    }
}
