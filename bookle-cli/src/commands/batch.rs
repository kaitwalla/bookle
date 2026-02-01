//! Batch conversion command implementation

use anyhow::{bail, Context, Result};
use bookle_core::decoder::decoder_for_extension;
use bookle_core::encoder::encoder_for_format;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Batch convert multiple ebooks
pub fn batch(input_dir: &str, output_dir: &str, format: &str, jobs: usize) -> Result<()> {
    let input_path = Path::new(input_dir);
    let output_path = Path::new(output_dir);

    // Ensure output directory exists
    fs::create_dir_all(output_path)?;

    // Find all supported files
    let files: Vec<_> = fs::read_dir(input_path)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|ext| decoder_for_extension(ext).is_some())
                .unwrap_or(false)
        })
        .collect();

    if files.is_empty() {
        println!("No supported files found in {}", input_dir);
        return Ok(());
    }

    println!("Found {} files to convert", files.len());

    // Get encoder
    let encoder = encoder_for_format(format)
        .with_context(|| format!("No encoder available for {} format", format))?;

    // Set up progress tracking
    let multi_progress = MultiProgress::new();
    let overall_pb = multi_progress.add(ProgressBar::new(files.len() as u64));
    overall_pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );

    let success_count = AtomicUsize::new(0);
    let error_count = AtomicUsize::new(0);

    // Configure thread pool
    rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build_global()
        .ok(); // Ignore if already configured

    // Process files in parallel
    files.par_iter().for_each(|file_path| {
        let result = process_file(file_path, output_path, &*encoder);

        match result {
            Ok(_) => {
                success_count.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                error_count.fetch_add(1, Ordering::Relaxed);
                tracing::error!("Failed to convert {:?}: {}", file_path, e);
            }
        }

        overall_pb.inc(1);
    });

    overall_pb.finish();

    let success = success_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);

    println!("\nBatch conversion complete:");
    println!("  Success: {}", success);
    println!("  Errors:  {}", errors);

    if errors > 0 {
        bail!("Batch conversion completed with {} errors", errors);
    }

    Ok(())
}

fn process_file(
    input_path: &Path,
    output_dir: &Path,
    encoder: &dyn bookle_core::encoder::Encoder,
) -> Result<()> {
    // Get decoder based on extension
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .context("Could not determine file extension")?;

    let decoder = decoder_for_extension(ext).context("No decoder available")?;

    // Read and decode
    let file = File::open(input_path)?;
    let mut reader = BufReader::new(file);
    let book = decoder.decode(&mut reader)?;

    // Build output path
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Could not determine output filename from input")?;
    let output_file = output_dir.join(format!("{}.{}", stem, encoder.file_extension()));

    // Encode
    let mut output = File::create(&output_file)?;
    encoder.encode(&book, &mut output)?;

    tracing::info!("Converted {:?} -> {:?}", input_path, output_file);

    Ok(())
}
