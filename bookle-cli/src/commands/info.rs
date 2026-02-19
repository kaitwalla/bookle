//! Info command implementation

use anyhow::{Context, Result};
use bookle_core::decoder::decoder_for_extension;
use serde::Serialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Book info output
#[derive(Serialize)]
struct BookInfo {
    title: String,
    authors: Vec<String>,
    language: String,
    description: Option<String>,
    publisher: Option<String>,
    chapters: usize,
    resources: usize,
}

/// Display information about an ebook
pub fn info(input: &str, json: bool) -> Result<()> {
    let input_path = Path::new(input);

    // Get file extension
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .context("Could not determine input file extension")?;

    // Get decoder
    let decoder = decoder_for_extension(ext)
        .with_context(|| format!("No decoder available for .{} files", ext))?;

    // Read and decode
    let file =
        File::open(input_path).with_context(|| format!("Failed to open input file: {}", input))?;
    let mut reader = BufReader::new(file);

    let book = decoder
        .decode(&mut reader)
        .with_context(|| format!("Failed to decode {}", input))?;

    let info = BookInfo {
        title: book.metadata.title.clone(),
        authors: book.metadata.creator.clone(),
        language: book.metadata.language.clone(),
        description: book.metadata.description.clone(),
        publisher: book.metadata.publisher.clone(),
        chapters: book.chapters.len(),
        resources: book.resources.len(),
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Title:       {}", info.title);
        if !info.authors.is_empty() {
            println!("Authors:     {}", info.authors.join(", "));
        }
        println!("Language:    {}", info.language);
        if let Some(desc) = &info.description {
            println!("Description: {}", desc);
        }
        if let Some(pub_) = &info.publisher {
            println!("Publisher:   {}", pub_);
        }
        println!("Chapters:    {}", info.chapters);
        println!("Resources:   {}", info.resources);
    }

    Ok(())
}
