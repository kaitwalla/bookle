//! Book metadata types (Dublin Core compliant with extensions)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Book metadata following Dublin Core standard with extensions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    /// Book title
    pub title: String,

    /// Authors/creators
    pub creator: Vec<String>,

    /// Subject/genre tags
    pub subject: Vec<String>,

    /// Book description/summary
    pub description: Option<String>,

    /// Publisher name
    pub publisher: Option<String>,

    /// Publication date
    pub date: Option<DateTime<Utc>>,

    /// Language code (ISO 639-1)
    pub language: String,

    /// ISBN or UUID identifier
    pub identifier: String,

    /// Resource key for cover image
    pub cover_resource_key: Option<String>,

    /// Series information
    pub series: Option<SeriesInfo>,

    /// Reading direction
    pub reading_direction: ReadingDirection,

    /// Copyright/rights information
    pub rights: Option<String>,
}

impl Metadata {
    /// Create new metadata with required fields
    pub fn new(title: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            creator: Vec::new(),
            subject: Vec::new(),
            description: None,
            publisher: None,
            date: None,
            language: language.into(),
            identifier: uuid::Uuid::new_v4().to_string(),
            cover_resource_key: None,
            series: None,
            reading_direction: ReadingDirection::LeftToRight,
            rights: None,
        }
    }

    /// Add an author/creator
    pub fn with_creator(mut self, creator: impl Into<String>) -> Self {
        self.creator.push(creator.into());
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set publisher
    pub fn with_publisher(mut self, publisher: impl Into<String>) -> Self {
        self.publisher = Some(publisher.into());
        self
    }
}

/// Series information for books that are part of a series
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeriesInfo {
    /// Series name
    pub name: String,

    /// Position in series (can be fractional for novellas, etc.)
    pub position: Option<f32>,
}

impl SeriesInfo {
    pub fn new(name: impl Into<String>, position: Option<f32>) -> Self {
        Self {
            name: name.into(),
            position,
        }
    }
}

/// Reading direction for the book
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ReadingDirection {
    /// Left to right (Latin, Cyrillic, etc.)
    #[default]
    LeftToRight,

    /// Right to left (Arabic, Hebrew, etc.)
    RightToLeft,

    /// Top to bottom (Traditional CJK)
    TopToBottom,
}
