//! LIT (Microsoft Reader) decoder implementation
//!
//! LIT is Microsoft's discontinued proprietary ebook format used by Microsoft Reader.
//! The format is based on ITOL/ITLS HTML Help 2.0 structure with LZX compression.
//!
//! Due to the complexity of the format (proprietary compression, compound document structure),
//! full parsing is limited. For complete LIT support, consider using external tools like
//! Calibre or ConvertLIT to convert to EPUB first.

use crate::error::ParseError;
use crate::types::{Block, Book, Chapter, Inline, Metadata};
use std::io::Read;

/// LIT file signature: "ITOLITLS" at the start of the file
const LIT_SIGNATURE: &[u8] = b"ITOLITLS";

/// Decoder for LIT (Microsoft Reader) format
pub struct LitDecoder {
    /// Whether to attempt content extraction (experimental)
    attempt_extraction: bool,
}

impl LitDecoder {
    pub fn new() -> Self {
        Self {
            attempt_extraction: false,
        }
    }

    /// Enable experimental content extraction
    pub fn with_extraction(mut self, enabled: bool) -> Self {
        self.attempt_extraction = enabled;
        self
    }

    /// Validate the LIT file signature
    fn validate_signature(&self, data: &[u8]) -> Result<(), ParseError> {
        if data.len() < LIT_SIGNATURE.len() {
            return Err(ParseError::MalformedContent(
                "File too small to be a valid LIT file".to_string(),
            ));
        }

        if &data[..LIT_SIGNATURE.len()] != LIT_SIGNATURE {
            return Err(ParseError::UnsupportedFormat(
                "Invalid LIT file signature".to_string(),
            ));
        }

        Ok(())
    }

    /// Extract basic metadata from LIT file structure
    /// The LIT format stores metadata in a specific location within the compound document
    fn extract_metadata(&self, data: &[u8]) -> Metadata {
        // LIT files store some metadata that can be extracted without full parsing
        // For now, return default metadata with a note about the format
        let mut metadata =
            Metadata::new("Unknown Title (LIT Format)".to_string(), "en".to_string());

        // Try to find title in the data (LIT stores it as UTF-16LE in some cases)
        if let Some(title) = self.try_extract_title(data) {
            metadata.title = title;
        }

        metadata.description = Some(
            "Imported from Microsoft Reader LIT format. For best results, \
             consider converting to EPUB using Calibre."
                .to_string(),
        );

        metadata
    }

    /// Attempt to extract title from LIT file
    /// LIT files sometimes have readable strings in the header area
    fn try_extract_title(&self, data: &[u8]) -> Option<String> {
        // LIT files contain the title in a known location after the header
        // The title is typically stored as UTF-16LE

        // Look for common patterns - this is a heuristic approach
        // The actual title location varies by LIT version

        // Search for readable title patterns in the first 4KB
        let search_area = &data[..std::cmp::min(4096, data.len())];

        // Try to find UTF-16LE strings
        self.find_utf16le_string(search_area, 10, 200)
    }

    /// Find a UTF-16LE encoded string in the data
    fn find_utf16le_string(&self, data: &[u8], min_len: usize, max_len: usize) -> Option<String> {
        // Look for sequences that could be UTF-16LE text
        // This is a simplified heuristic

        let mut i = 0;
        while i + 2 <= data.len() {
            // Look for a sequence of printable ASCII characters in UTF-16LE
            let mut chars = Vec::new();
            let mut j = i;

            while j + 2 <= data.len() {
                let lo = data[j];
                let hi = data[j + 1];

                // Check for ASCII printable character in UTF-16LE (hi byte = 0)
                if hi == 0 && lo >= 0x20 && lo <= 0x7E {
                    chars.push(lo as char);
                    j += 2;
                } else if !chars.is_empty() {
                    break;
                } else {
                    break;
                }
            }

            // If we found a reasonable string, check if it looks like a title
            if chars.len() >= min_len && chars.len() <= max_len {
                let s: String = chars.iter().collect();
                // Filter out things that don't look like titles
                if !s.contains("http")
                    && !s.contains("\\")
                    && !s.contains("/")
                    && !s
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_whitespace())
                {
                    return Some(s);
                }
            }

            i += 2;
        }

        None
    }

    /// Create a placeholder chapter for LIT files
    fn create_placeholder_chapter(&self) -> Chapter {
        let content = vec![
            Block::Header {
                level: 1,
                content: vec![Inline::Text("LIT Format Import".to_string())],
                anchor: None,
            },
            Block::Paragraph(vec![
                Inline::Text(
                    "This book was imported from Microsoft Reader's LIT format. ".to_string(),
                ),
                Inline::Text(
                    "Due to the proprietary nature of the LIT format, full content extraction \
                     is limited."
                        .to_string(),
                ),
            ]),
            Block::Paragraph(vec![
                Inline::Bold(vec![Inline::Text("Recommendation: ".to_string())]),
                Inline::Text(
                    "For complete book content, please convert this LIT file to EPUB using:"
                        .to_string(),
                ),
            ]),
            Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph(vec![
                        Inline::Bold(vec![Inline::Text("Calibre".to_string())]),
                        Inline::Text(" - Free, open-source ebook management software".to_string()),
                    ])],
                    vec![Block::Paragraph(vec![
                        Inline::Bold(vec![Inline::Text("ConvertLIT".to_string())]),
                        Inline::Text(" - Command-line tool for LIT conversion".to_string()),
                    ])],
                ],
            },
            Block::Paragraph(vec![Inline::Text(
                "After conversion to EPUB, you can re-import the book for full functionality."
                    .to_string(),
            )]),
        ];

        Chapter::new("LIT Format Information")
            .with_id("lit-info".to_string())
            .with_content(content)
    }
}

impl Default for LitDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Decoder for LitDecoder {
    fn decode(&self, reader: &mut dyn Read) -> Result<Book, ParseError> {
        // Read all data
        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .map_err(|e| ParseError::MalformedContent(format!("Failed to read LIT file: {}", e)))?;

        // Validate the LIT signature
        self.validate_signature(&data)?;

        // Extract what metadata we can
        let metadata = self.extract_metadata(&data);
        let mut book = Book::with_metadata(metadata);

        // Add a placeholder chapter explaining the LIT format limitations
        book.add_chapter(self.create_placeholder_chapter());

        // Note: Full LIT parsing would require:
        // 1. Parsing the ITOL/ITLS compound document structure
        // 2. Implementing LZX decompression
        // 3. Extracting the OEBPS content streams
        // 4. Handling potential DRM
        //
        // For now, we provide a valid Book structure with basic info

        Ok(book)
    }

    fn supported_extensions(&self) -> &[&str] {
        &["lit"]
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-ms-reader", "application/x-ms-lit"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::Decoder;

    #[test]
    fn test_validate_signature_valid() {
        let decoder = LitDecoder::new();
        let data = b"ITOLITLS\x00\x00\x00\x00";
        assert!(decoder.validate_signature(data).is_ok());
    }

    #[test]
    fn test_validate_signature_invalid() {
        let decoder = LitDecoder::new();
        let data = b"NOTVALID\x00\x00";
        assert!(decoder.validate_signature(data).is_err());
    }

    #[test]
    fn test_validate_signature_too_short() {
        let decoder = LitDecoder::new();
        let data = b"ITO";
        let result = decoder.validate_signature(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_supported_extensions() {
        let decoder = LitDecoder::new();
        assert!(decoder.supported_extensions().contains(&"lit"));
    }

    #[test]
    fn test_decode_creates_book() {
        let decoder = LitDecoder::new();

        // Create minimal valid LIT data
        let mut data = Vec::from(LIT_SIGNATURE);
        data.extend_from_slice(&[0u8; 100]);

        let mut cursor = std::io::Cursor::new(data);
        let result = decoder.decode(&mut cursor);

        assert!(result.is_ok());
        let book = result.unwrap();
        assert!(!book.chapters.is_empty());
    }

    #[test]
    fn test_find_utf16le_string() {
        let decoder = LitDecoder::new();

        // "Hello World" in UTF-16LE
        let data: Vec<u8> = vec![
            0x48, 0x00, 0x65, 0x00, 0x6C, 0x00, 0x6C, 0x00, 0x6F, 0x00, 0x20, 0x00, 0x57, 0x00,
            0x6F, 0x00, 0x72, 0x00, 0x6C, 0x00, 0x64, 0x00,
        ];

        let result = decoder.find_utf16le_string(&data, 5, 50);
        assert_eq!(result, Some("Hello World".to_string()));
    }

    #[test]
    fn test_placeholder_chapter() {
        let decoder = LitDecoder::new();
        let chapter = decoder.create_placeholder_chapter();

        assert_eq!(chapter.title, "LIT Format Information");
        assert!(!chapter.content.is_empty());
    }
}
