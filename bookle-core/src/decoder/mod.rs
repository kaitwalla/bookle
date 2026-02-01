//! Decoders for converting input formats to the IR

mod epub;
mod kepub;
mod lit;
mod markdown;
mod mobi;
mod pdf;

pub use epub::EpubDecoder;
pub use kepub::KepubDecoder;
pub use lit::LitDecoder;
pub use markdown::MarkdownDecoder;
pub use mobi::MobiDecoder;
pub use pdf::PdfDecoder;

use crate::error::ParseError;
use crate::types::Book;
use std::io::Read;

/// Trait for decoding ebook formats into the IR
pub trait Decoder: Send + Sync {
    /// Decode a book from a reader
    fn decode(&self, reader: &mut dyn Read) -> Result<Book, ParseError>;

    /// File extensions this decoder supports
    fn supported_extensions(&self) -> &[&str];

    /// MIME types this decoder supports
    fn supported_mime_types(&self) -> &[&str];
}

/// Get the appropriate decoder for a file extension
pub fn decoder_for_extension(ext: &str) -> Option<Box<dyn Decoder>> {
    // Handle compound extensions like "kepub.epub"
    let ext_lower = ext.to_lowercase();

    match ext_lower.as_str() {
        "kepub.epub" | "kepub" => Some(Box::new(KepubDecoder::new())),
        "epub" => Some(Box::new(EpubDecoder::new())),
        "lit" => Some(Box::new(LitDecoder::new())),
        "md" | "markdown" | "mdown" | "mkd" => Some(Box::new(MarkdownDecoder::new())),
        "pdf" => Some(Box::new(PdfDecoder::new())),
        "mobi" | "azw" | "azw3" | "prc" => Some(Box::new(MobiDecoder::new())),
        _ => None,
    }
}

/// Get the appropriate decoder for a MIME type
pub fn decoder_for_mime_type(mime: &str) -> Option<Box<dyn Decoder>> {
    match mime {
        "application/x-kobo-epub+zip" => Some(Box::new(KepubDecoder::new())),
        "application/epub+zip" => Some(Box::new(EpubDecoder::new())),
        "application/x-ms-reader" | "application/x-ms-lit" => Some(Box::new(LitDecoder::new())),
        "text/markdown" | "text/x-markdown" => Some(Box::new(MarkdownDecoder::new())),
        "application/pdf" => Some(Box::new(PdfDecoder::new())),
        "application/x-mobipocket-ebook" | "application/vnd.amazon.ebook" => {
            Some(Box::new(MobiDecoder::new()))
        }
        _ => None,
    }
}
