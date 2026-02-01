//! Chapter type representing a single chapter/section of a book

use super::Block;
use serde::{Deserialize, Serialize};

/// A single chapter of a book
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Chapter {
    /// Chapter title
    pub title: String,

    /// Optional chapter ID for cross-references
    pub id: Option<String>,

    /// The content blocks
    pub content: Vec<Block>,
}

impl Chapter {
    /// Create a new chapter with a title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            id: None,
            content: Vec::new(),
        }
    }

    /// Set the chapter ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add content blocks
    pub fn with_content(mut self, content: Vec<Block>) -> Self {
        self.content = content;
        self
    }

    /// Add a single block
    pub fn add_block(&mut self, block: Block) {
        self.content.push(block);
    }
}
