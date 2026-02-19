//! Semantic AST types for book content

use serde::{Deserialize, Serialize};

/// Block-level content element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Block {
    /// Heading (h1-h6)
    Header {
        level: u8,
        content: Vec<Inline>,
        anchor: Option<String>,
    },

    /// Paragraph of text
    Paragraph(Vec<Inline>),

    /// Ordered or unordered list
    List {
        items: Vec<Vec<Block>>,
        ordered: bool,
    },

    /// Image with optional caption
    Image {
        resource_key: String,
        caption: Option<String>,
        alt: String,
    },

    /// Code block with optional language
    CodeBlock { lang: Option<String>, code: String },

    /// Block quote
    Blockquote(Vec<Block>),

    /// Horizontal rule / thematic break
    ThematicBreak,

    /// Table
    Table(TableData),

    /// Footnote definition
    Footnote { id: String, content: Vec<Block> },
}

/// Inline content element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Inline {
    /// Plain text
    Text(String),

    /// Bold/strong text
    Bold(Vec<Inline>),

    /// Italic/emphasized text
    Italic(Vec<Inline>),

    /// Inline code
    Code(String),

    /// Hyperlink
    Link { children: Vec<Inline>, url: String },

    /// Superscript text
    Superscript(Vec<Inline>),

    /// Subscript text
    Subscript(Vec<Inline>),

    /// Strikethrough text
    Strikethrough(Vec<Inline>),

    /// Footnote reference
    FootnoteRef { id: String },

    /// Ruby annotation (for CJK texts)
    Ruby { base: String, annotation: String },

    /// Line break
    Break,
}

impl Inline {
    /// Create a plain text inline
    pub fn text(s: impl Into<String>) -> Self {
        Inline::Text(s.into())
    }

    /// Create a bold inline
    pub fn bold(children: Vec<Inline>) -> Self {
        Inline::Bold(children)
    }

    /// Create an italic inline
    pub fn italic(children: Vec<Inline>) -> Self {
        Inline::Italic(children)
    }

    /// Create a link inline
    pub fn link(children: Vec<Inline>, url: impl Into<String>) -> Self {
        Inline::Link {
            children,
            url: url.into(),
        }
    }
}

impl Block {
    /// Create a paragraph from inline elements
    pub fn paragraph(content: Vec<Inline>) -> Self {
        Block::Paragraph(content)
    }

    /// Create a header
    pub fn header(level: u8, content: Vec<Inline>) -> Self {
        Block::Header {
            level: level.min(6).max(1),
            content,
            anchor: None,
        }
    }

    /// Create a code block
    pub fn code_block(code: impl Into<String>, lang: Option<String>) -> Self {
        Block::CodeBlock {
            lang,
            code: code.into(),
        }
    }
}

/// Table data structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableData {
    /// Table header row
    pub headers: Vec<TableCell>,

    /// Table body rows
    pub rows: Vec<Vec<TableCell>>,
}

/// Single table cell
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableCell {
    /// Cell content
    pub content: Vec<Inline>,

    /// Column span
    pub colspan: u32,

    /// Row span
    pub rowspan: u32,
}

impl TableCell {
    pub fn new(content: Vec<Inline>) -> Self {
        Self {
            content,
            colspan: 1,
            rowspan: 1,
        }
    }
}
