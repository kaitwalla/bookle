//! PDF decoder implementation

use crate::error::ParseError;
use crate::types::{Block, Book, Chapter, Inline, Metadata};
use std::io::Read;

/// Decoder for PDF format
///
/// This decoder extracts text content from PDF files and converts it to the IR format.
/// Note that PDF is a visual format, so structural information (headings, lists, etc.)
/// is inferred heuristically from font sizes and spacing.
pub struct PdfDecoder {
    /// Minimum font size ratio to consider as a heading
    heading_size_ratio: f32,
}

impl PdfDecoder {
    pub fn new() -> Self {
        Self {
            heading_size_ratio: 1.2, // 20% larger than body text
        }
    }

    /// Set the heading size ratio threshold
    pub fn with_heading_ratio(mut self, ratio: f32) -> Self {
        self.heading_size_ratio = ratio;
        self
    }

    /// Extract text from PDF and convert to blocks
    fn extract_text_to_blocks(&self, data: &[u8]) -> Result<Vec<Block>, ParseError> {
        // Extract text from PDF
        let text = pdf_extract::extract_text_from_mem(data)
            .map_err(|e| ParseError::MalformedContent(format!("Failed to extract PDF text: {}", e)))?;

        // Split into paragraphs and convert to blocks
        let mut blocks = Vec::new();
        let mut current_para = String::new();

        for line in text.lines() {
            let line = line.trim();

            if line.is_empty() {
                // Empty line indicates paragraph break
                if !current_para.is_empty() {
                    blocks.push(self.text_to_block(&current_para));
                    current_para.clear();
                }
            } else {
                // Accumulate text
                if !current_para.is_empty() {
                    current_para.push(' ');
                }
                current_para.push_str(line);
            }
        }

        // Don't forget the last paragraph
        if !current_para.is_empty() {
            blocks.push(self.text_to_block(&current_para));
        }

        Ok(blocks)
    }

    /// Convert text to a block, detecting if it's likely a heading
    fn text_to_block(&self, text: &str) -> Block {
        let trimmed = text.trim();

        // Heuristics for heading detection:
        // 1. Short lines (likely chapter titles)
        // 2. Lines that look like numbered chapters
        // 3. ALL CAPS lines

        let is_likely_heading = self.is_likely_heading(trimmed);

        if is_likely_heading {
            let level = self.detect_heading_level(trimmed);
            Block::Header {
                level,
                content: vec![Inline::Text(trimmed.to_string())],
                anchor: None,
            }
        } else {
            Block::Paragraph(vec![Inline::Text(trimmed.to_string())])
        }
    }

    /// Detect if text is likely a heading based on heuristics
    fn is_likely_heading(&self, text: &str) -> bool {
        // Check various heading patterns

        // Pattern 1: Short text (< 100 chars) that doesn't end with common sentence enders
        let is_short = text.len() < 100 && !text.ends_with('.') && !text.ends_with('?') && !text.ends_with('!');

        // Pattern 2: Numbered chapter patterns
        let numbered_patterns = [
            "chapter", "part", "section", "book", "volume",
            "prologue", "epilogue", "introduction", "conclusion",
            "preface", "appendix", "foreword", "afterword",
        ];
        let text_lower = text.to_lowercase();
        let has_chapter_keyword = numbered_patterns.iter().any(|p| text_lower.starts_with(p));

        // Pattern 3: All uppercase (but not too long)
        let is_all_caps = text.len() < 60 && text.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase());

        // Pattern 4: Roman numeral or number at start
        let starts_with_number = text.chars().next().map(|c| c.is_numeric()).unwrap_or(false);
        let roman_numerals = ["I.", "II.", "III.", "IV.", "V.", "VI.", "VII.", "VIII.", "IX.", "X."];
        let starts_with_roman = roman_numerals.iter().any(|r| text.starts_with(r));

        is_short && (has_chapter_keyword || is_all_caps || starts_with_number || starts_with_roman)
    }

    /// Detect heading level based on text characteristics
    fn detect_heading_level(&self, text: &str) -> u8 {
        let text_lower = text.to_lowercase();

        // Level 1: Book, Part, Major divisions
        if text_lower.starts_with("book ") || text_lower.starts_with("part ") {
            return 1;
        }

        // Level 2: Chapters
        if text_lower.starts_with("chapter ") || text_lower.starts_with("prologue") ||
           text_lower.starts_with("epilogue") {
            return 2;
        }

        // Level 3: Sections
        if text_lower.starts_with("section ") {
            return 3;
        }

        // Default to level 2 for other headings
        2
    }

    /// Split blocks into chapters
    fn split_into_chapters(blocks: Vec<Block>) -> Vec<Chapter> {
        let mut chapters = Vec::new();
        let mut current_blocks = Vec::new();
        let mut current_title: Option<String> = None;

        for block in blocks {
            // Check if this is a chapter-level heading (level 1 or 2)
            let is_chapter_heading = matches!(&block, Block::Header { level, .. } if *level <= 2);

            if is_chapter_heading {
                // Save previous chapter if exists
                if !current_blocks.is_empty() || current_title.is_some() {
                    let title = current_title.take().unwrap_or_else(|| "Untitled".to_string());
                    chapters.push(Chapter::new(title).with_content(current_blocks));
                    current_blocks = Vec::new();
                }

                // Extract title from heading
                if let Block::Header { content, .. } = &block {
                    current_title = Some(inlines_to_text(content));
                }
                current_blocks.push(block);
            } else {
                current_blocks.push(block);
            }
        }

        // Don't forget the last chapter
        if !current_blocks.is_empty() || current_title.is_some() {
            let title = current_title.unwrap_or_else(|| "Untitled".to_string());
            chapters.push(Chapter::new(title).with_content(current_blocks));
        }

        // If no chapters were created, make a single chapter
        if chapters.is_empty() {
            chapters.push(Chapter::new("Content").with_content(vec![]));
        }

        chapters
    }

    /// Extract title from first heading or use filename
    fn extract_title(blocks: &[Block]) -> Option<String> {
        for block in blocks {
            if let Block::Header { content, level, .. } = block {
                if *level <= 2 {
                    return Some(inlines_to_text(content));
                }
            }
        }
        None
    }
}

impl Default for PdfDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Decoder for PdfDecoder {
    fn decode(&self, reader: &mut dyn Read) -> Result<Book, ParseError> {
        // Read all data into memory
        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .map_err(|e| ParseError::MalformedContent(format!("Failed to read PDF: {}", e)))?;

        // Extract text and convert to blocks
        let blocks = self.extract_text_to_blocks(&data)?;

        // Extract title from first heading
        let title = Self::extract_title(&blocks).unwrap_or_else(|| "Untitled PDF".to_string());

        // Create metadata
        let metadata = Metadata::new(title, "en");
        let mut book = Book::with_metadata(metadata);

        // Split into chapters
        let chapters = Self::split_into_chapters(blocks);
        for chapter in chapters {
            book.add_chapter(chapter);
        }

        Ok(book)
    }

    fn supported_extensions(&self) -> &[&str] {
        &["pdf"]
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/pdf"]
    }
}

/// Convert inline elements to plain text
fn inlines_to_text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|i| match i {
            Inline::Text(s) => s.clone(),
            Inline::Bold(children) | Inline::Italic(children) | Inline::Strikethrough(children) => {
                inlines_to_text(children)
            }
            Inline::Link { children, .. } => inlines_to_text(children),
            Inline::Code(s) => s.clone(),
            Inline::Superscript(children) | Inline::Subscript(children) => inlines_to_text(children),
            Inline::FootnoteRef { id } => format!("[{}]", id),
            Inline::Ruby { base, .. } => base.clone(),
            Inline::Break => " ".to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_likely_heading() {
        let decoder = PdfDecoder::new();

        assert!(decoder.is_likely_heading("Chapter 1"));
        assert!(decoder.is_likely_heading("INTRODUCTION"));
        assert!(decoder.is_likely_heading("Part I"));
        assert!(decoder.is_likely_heading("Prologue"));

        // Regular text should not be detected as heading
        assert!(!decoder.is_likely_heading("This is a regular paragraph that continues for a while and discusses various topics."));
    }

    #[test]
    fn test_detect_heading_level() {
        let decoder = PdfDecoder::new();

        assert_eq!(decoder.detect_heading_level("Book One"), 1);
        assert_eq!(decoder.detect_heading_level("Part I"), 1);
        assert_eq!(decoder.detect_heading_level("Chapter 1"), 2);
        assert_eq!(decoder.detect_heading_level("Prologue"), 2);
        assert_eq!(decoder.detect_heading_level("Section 3.1"), 3);
    }

    #[test]
    fn test_text_to_block_paragraph() {
        let decoder = PdfDecoder::new();

        let block = decoder.text_to_block("This is a regular paragraph with normal text content.");

        match block {
            Block::Paragraph(inlines) => {
                assert_eq!(inlines.len(), 1);
            }
            _ => panic!("Expected paragraph"),
        }
    }

    #[test]
    fn test_text_to_block_heading() {
        let decoder = PdfDecoder::new();

        let block = decoder.text_to_block("Chapter 1");

        match block {
            Block::Header { level, .. } => {
                assert_eq!(level, 2);
            }
            _ => panic!("Expected header"),
        }
    }
}
