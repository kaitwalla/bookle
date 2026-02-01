//! Table of contents types

use serde::{Deserialize, Serialize};

/// A single entry in the table of contents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TocEntry {
    /// Display title
    pub title: String,

    /// Target anchor/ID within the book
    pub href: String,

    /// Nesting level (0 = top level)
    pub level: u32,

    /// Child entries for nested TOC
    pub children: Vec<TocEntry>,
}

impl TocEntry {
    /// Create a new TOC entry
    pub fn new(title: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            href: href.into(),
            level: 0,
            children: Vec::new(),
        }
    }

    /// Set the nesting level
    pub fn with_level(mut self, level: u32) -> Self {
        self.level = level;
        self
    }

    /// Add child entries
    pub fn with_children(mut self, children: Vec<TocEntry>) -> Self {
        self.children = children;
        self
    }

    /// Add a single child entry
    pub fn add_child(&mut self, child: TocEntry) {
        self.children.push(child);
    }
}
