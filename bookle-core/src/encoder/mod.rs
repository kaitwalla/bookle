//! Encoders for converting the IR to output formats

mod epub;
mod kepub;
mod typst_pdf;

pub use epub::EpubEncoder;
pub use kepub::KepubEncoder;
pub use typst_pdf::TypstPdfEncoder;

use crate::error::ConversionError;
use crate::types::Book;
use std::io::Write;

/// Trait for encoding books to output formats
pub trait Encoder: Send + Sync {
    /// Encode a book to a writer
    fn encode(&self, book: &Book, writer: &mut dyn Write) -> Result<(), ConversionError>;

    /// Format name (e.g., "EPUB", "PDF")
    fn format_name(&self) -> &str;

    /// File extension for this format
    fn file_extension(&self) -> &str;

    /// MIME type for this format
    fn mime_type(&self) -> &str;
}

/// Get an encoder by format name
pub fn encoder_for_format(format: &str) -> Option<Box<dyn Encoder>> {
    match format.to_lowercase().as_str() {
        "epub" => Some(Box::new(EpubEncoder::new())),
        "kepub" | "kepub.epub" => Some(Box::new(KepubEncoder::new())),
        "pdf" | "typ" | "typst" => Some(Box::new(TypstPdfEncoder::new())),
        _ => None,
    }
}
