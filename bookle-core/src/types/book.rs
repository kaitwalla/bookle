//! The main Book type - the root of the IR

use super::{Chapter, Metadata, ResourceStore, TocEntry};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The complete book representation
/// This is the core Intermediate Representation (IR) that all formats convert to/from
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Book {
    /// Unique identifier for this book
    pub id: Uuid,

    /// Book metadata (title, author, etc.)
    pub metadata: Metadata,

    /// Ordered list of chapters
    pub chapters: Vec<Chapter>,

    /// Content-addressed resource store (images, fonts, etc.)
    pub resources: ResourceStore,

    /// Table of contents
    pub toc: Vec<TocEntry>,
}

impl Book {
    /// Create a new book with the given title and language
    pub fn new(title: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            metadata: Metadata::new(title, language),
            chapters: Vec::new(),
            resources: ResourceStore::new(),
            toc: Vec::new(),
        }
    }

    /// Create a book with existing metadata
    pub fn with_metadata(metadata: Metadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            metadata,
            chapters: Vec::new(),
            resources: ResourceStore::new(),
            toc: Vec::new(),
        }
    }

    /// Add a chapter to the book
    pub fn add_chapter(&mut self, chapter: Chapter) {
        self.chapters.push(chapter);
    }

    /// Add a TOC entry
    pub fn add_toc_entry(&mut self, entry: TocEntry) {
        self.toc.push(entry);
    }

    /// Get the book title
    pub fn title(&self) -> &str {
        &self.metadata.title
    }

    /// Get the primary author (first creator)
    pub fn primary_author(&self) -> Option<&str> {
        self.metadata.creator.first().map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Block, Inline};

    #[test]
    fn test_book_creation() {
        let mut book = Book::new("Test Book", "en");
        assert_eq!(book.title(), "Test Book");
        assert_eq!(book.metadata.language, "en");
        assert!(book.chapters.is_empty());

        let mut chapter = Chapter::new("Chapter 1");
        chapter.add_block(Block::paragraph(vec![Inline::text("Hello, world!")]));
        book.add_chapter(chapter);

        assert_eq!(book.chapters.len(), 1);
        assert_eq!(book.chapters[0].title, "Chapter 1");
    }

    #[test]
    fn test_book_serialization() {
        let book = Book::new("Serialization Test", "en");
        let json = serde_json::to_string(&book).unwrap();
        let deserialized: Book = serde_json::from_str(&json).unwrap();
        assert_eq!(book.metadata.title, deserialized.metadata.title);
    }
}
