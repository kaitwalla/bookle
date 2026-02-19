//! KEPUB decoder implementation
//!
//! KEPUB is Kobo's proprietary EPUB variant. It uses the same structure as EPUB
//! but adds Kobo-specific span elements for reading position tracking.
//! This decoder strips out Kobo-specific markup and delegates to the EPUB decoder.

use crate::error::ParseError;
use crate::types::Book;
use std::io::{Cursor, Read};

/// Decoder for KEPUB (Kobo EPUB) format
pub struct KepubDecoder {
    epub_decoder: super::EpubDecoder,
}

impl KepubDecoder {
    pub fn new() -> Self {
        Self {
            epub_decoder: super::EpubDecoder::new(),
        }
    }

    /// Preprocess KEPUB content to remove Kobo-specific markup
    fn preprocess_kepub_data(&self, data: &[u8]) -> Result<Vec<u8>, ParseError> {
        use std::io::{Read as _, Write};
        use zip::ZipArchive;

        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| ParseError::InvalidEpub(format!("Invalid KEPUB archive: {}", e)))?;

        // Create a new ZIP with processed content
        let mut output = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut output));
            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            for i in 0..archive.len() {
                let mut file = archive
                    .by_index(i)
                    .map_err(|e| ParseError::InvalidEpub(e.to_string()))?;

                let name = file.name().to_string();

                // Read file content
                let mut content = Vec::new();
                file.read_to_end(&mut content)
                    .map_err(|e| ParseError::InvalidEpub(e.to_string()))?;

                // Process HTML/XHTML files to strip Kobo spans
                let processed = if name.ends_with(".xhtml")
                    || name.ends_with(".html")
                    || name.ends_with(".htm")
                {
                    let html = String::from_utf8_lossy(&content);
                    let cleaned = self.clean_kobo_markup(&html);
                    cleaned.into_bytes()
                } else {
                    content
                };

                writer
                    .start_file(&name, options)
                    .map_err(|e| ParseError::InvalidEpub(e.to_string()))?;
                writer
                    .write_all(&processed)
                    .map_err(|e| ParseError::InvalidEpub(e.to_string()))?;
            }

            writer
                .finish()
                .map_err(|e| ParseError::InvalidEpub(e.to_string()))?;
        }

        Ok(output)
    }

    /// Clean Kobo-specific markup from HTML content
    fn clean_kobo_markup(&self, html: &str) -> String {
        // Use a simple state machine approach to remove koboSpan elements
        // while preserving their content
        self.remove_kobo_spans_simple(html)
    }

    /// Simple span removal that preserves content
    fn remove_kobo_spans_simple(&self, html: &str) -> String {
        let mut result = String::with_capacity(html.len());
        let mut chars = html.chars().peekable();
        let mut in_tag = false;
        let mut current_tag = String::new();
        let mut skip_closing_spans = 0;

        while let Some(c) = chars.next() {
            if c == '<' {
                in_tag = true;
                current_tag.clear();
                current_tag.push(c);
            } else if in_tag {
                current_tag.push(c);
                if c == '>' {
                    in_tag = false;

                    // Check if this is a koboSpan opening tag
                    if current_tag.contains("koboSpan")
                        || (current_tag.contains("id=\"kobo.") && current_tag.starts_with("<span"))
                    {
                        // Skip this tag, increment counter for closing span
                        skip_closing_spans += 1;
                    } else if current_tag == "</span>" && skip_closing_spans > 0 {
                        // Skip this closing tag
                        skip_closing_spans -= 1;
                    } else {
                        result.push_str(&current_tag);
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }
}

impl Default for KepubDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Decoder for KepubDecoder {
    fn decode(&self, reader: &mut dyn Read) -> Result<Book, ParseError> {
        // Read all data
        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .map_err(|e| ParseError::InvalidEpub(format!("Failed to read KEPUB: {}", e)))?;

        // Preprocess to remove Kobo-specific markup
        let processed_data = self.preprocess_kepub_data(&data)?;

        // Delegate to EPUB decoder
        let mut cursor = Cursor::new(processed_data);
        self.epub_decoder.decode(&mut cursor)
    }

    fn supported_extensions(&self) -> &[&str] {
        // KEPUB files typically have .kepub.epub extension
        // We'll handle this specially in the decoder factory
        &["kepub.epub", "kepub"]
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-kobo-epub+zip", "application/epub+zip"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::Decoder;

    #[test]
    fn test_clean_kobo_markup() {
        let decoder = KepubDecoder::new();

        let html = r#"<p><span class="koboSpan" id="kobo.1.1">Hello </span><span class="koboSpan" id="kobo.1.2">world</span></p>"#;
        let cleaned = decoder.clean_kobo_markup(html);

        assert!(!cleaned.contains("koboSpan"));
        assert!(cleaned.contains("Hello"));
        assert!(cleaned.contains("world"));
    }

    #[test]
    fn test_remove_kobo_spans_simple() {
        let decoder = KepubDecoder::new();

        let html = r#"<p><span id="kobo.1.1" class="koboSpan">Text here</span></p>"#;
        let result = decoder.remove_kobo_spans_simple(html);

        assert_eq!(result, "<p>Text here</p>");
    }

    #[test]
    fn test_nested_spans() {
        let decoder = KepubDecoder::new();

        let html =
            r#"<p><span class="koboSpan" id="kobo.1.1"><strong>Bold</strong> text</span></p>"#;
        let result = decoder.remove_kobo_spans_simple(html);

        assert!(result.contains("<strong>Bold</strong>"));
        assert!(!result.contains("koboSpan"));
    }

    #[test]
    fn test_supported_extensions() {
        let decoder = KepubDecoder::new();
        assert!(decoder.supported_extensions().contains(&"kepub.epub"));
        assert!(decoder.supported_extensions().contains(&"kepub"));
    }
}
