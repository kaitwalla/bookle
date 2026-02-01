//! Bookle Core Library
//!
//! This crate provides the core types and conversion logic for the Bookle ebook management system.
//! All ebook formats are converted to an Intermediate Representation (IR) before being
//! encoded to target formats.

pub mod decoder;
pub mod encoder;
pub mod error;
pub mod storage;
pub mod types;

pub use error::{BookleError, ConversionError, ParseError, Result};
pub use types::{
    Block, Book, Chapter, Inline, Metadata, ReadingDirection, Resource, ResourceData,
    ResourceStore, SeriesInfo, TableCell, TableData, TocEntry,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_creation() {
        let book = Book::new("Test Book", "en");
        assert_eq!(book.metadata.title, "Test Book");
        assert_eq!(book.metadata.language, "en");
    }
}
