//! KEPUB encoder implementation
//!
//! KEPUB is Kobo's proprietary EPUB variant. It wraps text content in
//! special spans for reading position tracking on Kobo devices.

use crate::error::ConversionError;
use crate::types::{Block, Book, Inline};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Encoder for KEPUB (Kobo EPUB) format
pub struct KepubEncoder {
    /// Chapter counter for span IDs
    chapter_counter: AtomicUsize,
    /// Span counter within chapter
    span_counter: AtomicUsize,
}

impl KepubEncoder {
    pub fn new() -> Self {
        Self {
            chapter_counter: AtomicUsize::new(1),
            span_counter: AtomicUsize::new(1),
        }
    }

    /// Reset counters for a new chapter
    fn reset_chapter(&self) {
        self.chapter_counter.fetch_add(1, Ordering::Relaxed);
        self.span_counter.store(1, Ordering::Relaxed);
    }

    /// Get the next koboSpan ID
    fn next_span_id(&self) -> String {
        let chapter = self.chapter_counter.load(Ordering::Relaxed);
        let span = self.span_counter.fetch_add(1, Ordering::Relaxed);
        format!("kobo.{}.{}", chapter, span)
    }

    /// Wrap text in a koboSpan
    fn wrap_kobo_span(&self, content: &str) -> String {
        if content.trim().is_empty() {
            return content.to_string();
        }
        let id = self.next_span_id();
        format!(r#"<span class="koboSpan" id="{}">{}</span>"#, id, content)
    }

    /// Convert Block AST to XHTML with Kobo spans
    fn blocks_to_xhtml(&self, blocks: &[Block]) -> String {
        let mut html = String::new();
        for block in blocks {
            html.push_str(&self.block_to_xhtml(block));
        }
        html
    }

    /// Convert a single Block to XHTML with Kobo spans
    fn block_to_xhtml(&self, block: &Block) -> String {
        match block {
            Block::Header {
                level,
                content,
                anchor,
            } => {
                let id_attr = anchor
                    .as_ref()
                    .map(|a| format!(" id=\"{}\"", escape_html(a)))
                    .unwrap_or_default();
                format!(
                    "<h{level}{id_attr}>{}</h{level}>\n",
                    self.inlines_to_xhtml(content)
                )
            }
            Block::Paragraph(inlines) => {
                format!("<p>{}</p>\n", self.inlines_to_xhtml(inlines))
            }
            Block::List { items, ordered } => {
                let tag = if *ordered { "ol" } else { "ul" };
                let items_html: String = items
                    .iter()
                    .map(|item| format!("<li>{}</li>", self.blocks_to_xhtml(item)))
                    .collect();
                format!("<{tag}>{items_html}</{tag}>\n")
            }
            Block::Image {
                resource_key,
                caption,
                alt,
            } => {
                let src_attr = escape_html(resource_key);
                let alt_attr = escape_html(alt);
                let img = format!("<img src=\"{}\" alt=\"{}\"/>", src_attr, alt_attr);
                if let Some(cap) = caption {
                    format!(
                        "<figure>{}<figcaption>{}</figcaption></figure>\n",
                        img,
                        self.wrap_kobo_span(&escape_html(cap))
                    )
                } else {
                    format!("{}\n", img)
                }
            }
            Block::CodeBlock { lang, code } => {
                let class_attr = lang
                    .as_ref()
                    .map(|l| format!(" class=\"language-{}\"", l))
                    .unwrap_or_default();
                format!("<pre><code{}>{}</code></pre>\n", class_attr, escape_html(code))
            }
            Block::Blockquote(blocks) => {
                format!("<blockquote>{}</blockquote>\n", self.blocks_to_xhtml(blocks))
            }
            Block::ThematicBreak => "<hr/>\n".to_string(),
            Block::Table(table) => {
                let mut html = String::from("<table>\n");
                if !table.headers.is_empty() {
                    html.push_str("<thead><tr>");
                    for cell in &table.headers {
                        html.push_str(&format!("<th>{}</th>", self.inlines_to_xhtml(&cell.content)));
                    }
                    html.push_str("</tr></thead>\n");
                }
                html.push_str("<tbody>");
                for row in &table.rows {
                    html.push_str("<tr>");
                    for cell in row {
                        html.push_str(&format!("<td>{}</td>", self.inlines_to_xhtml(&cell.content)));
                    }
                    html.push_str("</tr>");
                }
                html.push_str("</tbody></table>\n");
                html
            }
            Block::Footnote { id, content } => {
                format!(
                    "<aside id=\"fn-{}\" epub:type=\"footnote\">{}</aside>\n",
                    escape_html(id),
                    self.blocks_to_xhtml(content)
                )
            }
        }
    }

    /// Convert inline elements to XHTML with Kobo spans
    fn inlines_to_xhtml(&self, inlines: &[Inline]) -> String {
        let mut html = String::new();
        for inline in inlines {
            html.push_str(&self.inline_to_xhtml(inline));
        }
        html
    }

    /// Convert a single Inline to XHTML with Kobo spans
    fn inline_to_xhtml(&self, inline: &Inline) -> String {
        match inline {
            Inline::Text(s) => {
                // Wrap text in koboSpan
                self.wrap_kobo_span(&escape_html(s))
            }
            Inline::Bold(children) => {
                format!("<strong>{}</strong>", self.inlines_to_xhtml(children))
            }
            Inline::Italic(children) => {
                format!("<em>{}</em>", self.inlines_to_xhtml(children))
            }
            Inline::Code(s) => format!("<code>{}</code>", escape_html(s)),
            Inline::Link { children, url } => {
                format!(
                    "<a href=\"{}\">{}</a>",
                    escape_html(url),
                    self.inlines_to_xhtml(children)
                )
            }
            Inline::Superscript(children) => {
                format!("<sup>{}</sup>", self.inlines_to_xhtml(children))
            }
            Inline::Subscript(children) => {
                format!("<sub>{}</sub>", self.inlines_to_xhtml(children))
            }
            Inline::Strikethrough(children) => {
                format!("<del>{}</del>", self.inlines_to_xhtml(children))
            }
            Inline::FootnoteRef { id } => {
                format!(
                    "<a href=\"#fn-{}\" epub:type=\"noteref\">[{}]</a>",
                    escape_html(id),
                    escape_html(id)
                )
            }
            Inline::Ruby { base, annotation } => {
                format!(
                    "<ruby>{}<rp>(</rp><rt>{}</rt><rp>)</rp></ruby>",
                    self.wrap_kobo_span(&escape_html(base)),
                    escape_html(annotation)
                )
            }
            Inline::Break => "<br/>".to_string(),
        }
    }

    /// Generate XHTML document for a chapter
    fn chapter_to_xhtml(&self, title: &str, content: &[Block]) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head>
    <title>{}</title>
    <meta charset="UTF-8"/>
    <style type="text/css">
        .koboSpan {{ }}
    </style>
</head>
<body>
{}
</body>
</html>"#,
            escape_html(title),
            self.blocks_to_xhtml(content)
        )
    }
}

impl Default for KepubEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Encoder for KepubEncoder {
    fn encode(&self, book: &Book, writer: &mut dyn Write) -> Result<(), ConversionError> {
        use epub_builder::{EpubBuilder, EpubContent, ZipLibrary};

        // Reset chapter counter for this encoding
        self.chapter_counter.store(0, Ordering::Relaxed);
        self.span_counter.store(1, Ordering::Relaxed);

        let mut builder = EpubBuilder::new(ZipLibrary::new().map_err(|e| {
            ConversionError::EncodingFailed(format!("Failed to create zip: {}", e))
        })?).map_err(|e| {
            ConversionError::EncodingFailed(format!("Failed to create EPUB builder: {}", e))
        })?;

        // Set metadata
        builder
            .metadata("title", &book.metadata.title)
            .map_err(|e| ConversionError::EncodingFailed(e.to_string()))?;

        for creator in &book.metadata.creator {
            builder
                .metadata("author", creator)
                .map_err(|e| ConversionError::EncodingFailed(e.to_string()))?;
        }

        builder
            .metadata("lang", &book.metadata.language)
            .map_err(|e| ConversionError::EncodingFailed(e.to_string()))?;

        // Add resources (images, fonts, etc.)
        for (key, resource) in book.resources.iter() {
            let data = resource.data.as_bytes()
                .map_err(|e| ConversionError::EncodingFailed(format!("Failed to read resource: {}", e)))?;
            let mime = &resource.mime_type;

            // Determine filename from key or original filename
            let filename = resource.original_filename.as_ref()
                .map(|f| format!("images/{}", f))
                .unwrap_or_else(|| format!("images/{}", key));

            builder
                .add_resource(&filename, data.as_slice(), mime)
                .map_err(|e| ConversionError::EncodingFailed(e.to_string()))?;
        }

        // Add chapters with Kobo spans
        for (i, chapter) in book.chapters.iter().enumerate() {
            self.reset_chapter();
            let xhtml = self.chapter_to_xhtml(&chapter.title, &chapter.content);
            let filename = format!("chapter_{}.xhtml", i + 1);

            builder
                .add_content(
                    EpubContent::new(&filename, xhtml.as_bytes())
                        .title(&chapter.title)
                        .reftype(epub_builder::ReferenceType::Text),
                )
                .map_err(|e| ConversionError::EncodingFailed(e.to_string()))?;
        }

        // Generate EPUB
        builder
            .generate(writer)
            .map_err(|e| ConversionError::EncodingFailed(e.to_string()))?;

        Ok(())
    }

    fn format_name(&self) -> &str {
        "KEPUB"
    }

    fn file_extension(&self) -> &str {
        "kepub.epub"
    }

    fn mime_type(&self) -> &str {
        "application/x-kobo-epub+zip"
    }
}

/// Escape HTML special characters
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kobo_span_generation() {
        let encoder = KepubEncoder::new();

        let span1 = encoder.wrap_kobo_span("Hello");
        assert!(span1.contains("koboSpan"));
        assert!(span1.contains("kobo.1.1"));
        assert!(span1.contains("Hello"));

        let span2 = encoder.wrap_kobo_span("World");
        assert!(span2.contains("kobo.1.2"));
    }

    #[test]
    fn test_chapter_reset() {
        let encoder = KepubEncoder::new();

        encoder.wrap_kobo_span("First");
        encoder.reset_chapter();
        let span = encoder.wrap_kobo_span("New chapter");

        assert!(span.contains("kobo.2.1"));
    }

    #[test]
    fn test_paragraph_with_kobo_spans() {
        let encoder = KepubEncoder::new();
        let block = Block::Paragraph(vec![
            Inline::Text("Hello ".to_string()),
            Inline::Bold(vec![Inline::Text("world".to_string())]),
        ]);

        let html = encoder.block_to_xhtml(&block);
        assert!(html.contains("<p>"));
        assert!(html.contains("koboSpan"));
        assert!(html.contains("<strong>"));
    }

    #[test]
    fn test_empty_text_not_wrapped() {
        let encoder = KepubEncoder::new();

        let span = encoder.wrap_kobo_span("  ");
        assert!(!span.contains("koboSpan"));
    }
}
