//! Typst-based PDF encoder
//!
//! This encoder converts books to Typst markup. The output can be:
//! - Compiled to PDF using the `typst` CLI tool
//! - Directly rendered by Typst-compatible viewers
//!
//! Future versions will include direct PDF compilation.

use crate::error::ConversionError;
use crate::types::{Block, Book, Inline};
use std::io::Write;

/// PDF encoder using Typst
///
/// Currently outputs Typst source markup (.typ files).
/// Use the `typst compile` CLI command to generate PDFs.
pub struct TypstPdfEncoder {
    /// Page configuration
    pub page_config: PageConfig,
    /// Whether to output raw Typst source (true) or attempt PDF compilation (false)
    pub output_source: bool,
}

/// Page configuration for PDF output
#[derive(Debug, Clone)]
pub struct PageConfig {
    /// Page width (e.g., "210mm" for A4)
    pub width: String,
    /// Page height (e.g., "297mm" for A4)
    pub height: String,
    /// Top margin
    pub margin_top: String,
    /// Bottom margin
    pub margin_bottom: String,
    /// Left margin
    pub margin_left: String,
    /// Right margin
    pub margin_right: String,
    /// Base font size
    pub font_size: String,
}

impl Default for PageConfig {
    fn default() -> Self {
        Self {
            width: "210mm".to_string(),
            height: "297mm".to_string(),
            margin_top: "2.5cm".to_string(),
            margin_bottom: "2.5cm".to_string(),
            margin_left: "2cm".to_string(),
            margin_right: "2cm".to_string(),
            font_size: "11pt".to_string(),
        }
    }
}

impl TypstPdfEncoder {
    pub fn new() -> Self {
        Self {
            page_config: PageConfig::default(),
            output_source: true, // Default to source output for now
        }
    }

    /// Set page configuration
    pub fn with_page_config(mut self, config: PageConfig) -> Self {
        self.page_config = config;
        self
    }

    /// Set whether to output Typst source directly
    pub fn with_output_source(mut self, source: bool) -> Self {
        self.output_source = source;
        self
    }

    /// Convert Book to Typst markup
    pub fn book_to_typst(&self, book: &Book) -> String {
        let mut typst = String::new();

        // Document setup
        typst.push_str(&format!(
            r#"#set page(
  width: {},
  height: {},
  margin: (
    top: {},
    bottom: {},
    left: {},
    right: {},
  ),
)

#set text(size: {})
#set heading(numbering: "1.1")

"#,
            self.page_config.width,
            self.page_config.height,
            self.page_config.margin_top,
            self.page_config.margin_bottom,
            self.page_config.margin_left,
            self.page_config.margin_right,
            self.page_config.font_size,
        ));

        // Title page
        typst.push_str(&format!(
            r#"#align(center)[
  #v(30%)
  #text(size: 24pt, weight: "bold")[{}]
  #v(1em)
"#,
            escape_typst(&book.metadata.title)
        ));

        for author in &book.metadata.creator {
            typst.push_str(&format!(
                "  #text(size: 14pt)[{}]\n",
                escape_typst(author)
            ));
        }

        typst.push_str("]\n\n#pagebreak()\n\n");

        // Table of contents (if there are chapters)
        if !book.chapters.is_empty() {
            typst.push_str("#outline(title: \"Contents\", depth: 2)\n\n#pagebreak()\n\n");
        }

        // Chapters
        for chapter in &book.chapters {
            typst.push_str(&format!("= {}\n\n", escape_typst(&chapter.title)));
            typst.push_str(&self.blocks_to_typst(&chapter.content));
            typst.push_str("\n#pagebreak()\n\n");
        }

        typst
    }

    /// Convert blocks to Typst
    fn blocks_to_typst(&self, blocks: &[Block]) -> String {
        let mut typst = String::new();
        for block in blocks {
            typst.push_str(&self.block_to_typst(block));
            typst.push('\n');
        }
        typst
    }

    /// Convert a single Block to Typst
    fn block_to_typst(&self, block: &Block) -> String {
        match block {
            Block::Header { level, content, anchor } => {
                let prefix = "=".repeat((*level as usize).min(6));
                let label = anchor
                    .as_ref()
                    .map(|a| format!(" <{}>", a))
                    .unwrap_or_default();
                format!("{} {}{}\n", prefix, self.inlines_to_typst(content), label)
            }
            Block::Paragraph(inlines) => {
                format!("{}\n", self.inlines_to_typst(inlines))
            }
            Block::List { items, ordered } => {
                let mut typst = String::new();
                for (i, item) in items.iter().enumerate() {
                    let marker = if *ordered {
                        format!("{}. ", i + 1)
                    } else {
                        "- ".to_string()
                    };
                    let content = self.blocks_to_typst(item).trim().to_string();
                    typst.push_str(&format!("{}{}\n", marker, content));
                }
                typst
            }
            Block::Image { resource_key, caption, .. } => {
                let mut typst = format!("#figure(\n  image(\"{}\", width: 80%),\n", resource_key);
                if let Some(cap) = caption {
                    typst.push_str(&format!("  caption: [{}],\n", escape_typst(cap)));
                }
                typst.push_str(")\n");
                typst
            }
            Block::CodeBlock { lang, code } => {
                let lang_str = lang.as_deref().unwrap_or("");
                // Use Typst raw block if code contains backticks
                if code.contains("```") {
                    let escaped_code = code.replace(']', "\\]");
                    if lang_str.is_empty() {
                        format!("#raw(block: true)[{}]\n", escaped_code)
                    } else {
                        format!("#raw(block: true, lang: \"{}\")[{}]\n", lang_str, escaped_code)
                    }
                } else {
                    format!("```{}\n{}\n```\n", lang_str, code)
                }
            }
            Block::Blockquote(blocks) => {
                let content = self.blocks_to_typst(blocks);
                format!("#quote(block: true)[\n{}\n]\n", content)
            }
            Block::ThematicBreak => {
                "#line(length: 100%)\n".to_string()
            }
            Block::Table(table) => {
                let cols = table.headers.len().max(
                    table.rows.first().map(|r| r.len()).unwrap_or(0)
                );
                let mut typst = format!("#table(\n  columns: {},\n", cols);

                // Headers
                for cell in &table.headers {
                    typst.push_str(&format!(
                        "  [*{}*],\n",
                        self.inlines_to_typst(&cell.content)
                    ));
                }

                // Rows
                for row in &table.rows {
                    for cell in row {
                        typst.push_str(&format!(
                            "  [{}],\n",
                            self.inlines_to_typst(&cell.content)
                        ));
                    }
                }

                typst.push_str(")\n");
                typst
            }
            Block::Footnote { id, content } => {
                format!(
                    "#footnote[{}] <fn-{}>\n",
                    self.blocks_to_typst(content).trim(),
                    id
                )
            }
        }
    }

    /// Convert inlines to Typst
    fn inlines_to_typst(&self, inlines: &[Inline]) -> String {
        let mut typst = String::new();
        for inline in inlines {
            typst.push_str(&self.inline_to_typst(inline));
        }
        typst
    }

    /// Convert a single Inline to Typst
    fn inline_to_typst(&self, inline: &Inline) -> String {
        match inline {
            Inline::Text(s) => escape_typst(s),
            Inline::Bold(children) => {
                format!("*{}*", self.inlines_to_typst(children))
            }
            Inline::Italic(children) => {
                format!("_{}_", self.inlines_to_typst(children))
            }
            Inline::Code(s) => {
                // Handle code containing backticks
                if s.contains('`') {
                    format!("#raw(\"{}\")", s.replace('"', "\\\""))
                } else {
                    format!("`{}`", s)
                }
            }
            Inline::Link { children, url } => {
                // Escape quotes in URL
                let escaped_url = url.replace('"', "\\\"");
                format!("#link(\"{}\")[{}]", escaped_url, self.inlines_to_typst(children))
            }
            Inline::Superscript(children) => {
                format!("#super[{}]", self.inlines_to_typst(children))
            }
            Inline::Subscript(children) => {
                format!("#sub[{}]", self.inlines_to_typst(children))
            }
            Inline::Strikethrough(children) => {
                format!("#strike[{}]", self.inlines_to_typst(children))
            }
            Inline::FootnoteRef { id } => {
                format!("#footnote[See footnote {}]", id)
            }
            Inline::Ruby { base, annotation } => {
                // Typst doesn't have native ruby support, use a workaround
                format!("{}#super[#text(size: 0.6em)[{}]]", escape_typst(base), escape_typst(annotation))
            }
            Inline::Break => "\\\n".to_string(),
        }
    }
}

impl Default for TypstPdfEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Encoder for TypstPdfEncoder {
    fn encode(&self, book: &Book, writer: &mut dyn Write) -> Result<(), ConversionError> {
        let typst_source = self.book_to_typst(book);

        // Output Typst source
        // Users can compile to PDF using: typst compile output.typ output.pdf
        writer
            .write_all(typst_source.as_bytes())
            .map_err(|e| ConversionError::EncodingFailed(e.to_string()))?;

        Ok(())
    }

    fn format_name(&self) -> &str {
        "Typst"
    }

    fn file_extension(&self) -> &str {
        "typ"
    }

    fn mime_type(&self) -> &str {
        "text/x-typst"
    }
}

/// Escape special Typst characters
fn escape_typst(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('#', "\\#")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('@', "\\@")
        .replace('$', "\\$")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph_to_typst() {
        let encoder = TypstPdfEncoder::new();
        let block = Block::Paragraph(vec![
            Inline::Text("Hello ".to_string()),
            Inline::Bold(vec![Inline::Text("world".to_string())]),
        ]);

        let typst = encoder.block_to_typst(&block);
        assert!(typst.contains("Hello"));
        assert!(typst.contains("*world*"));
    }

    #[test]
    fn test_escape_typst() {
        assert_eq!(escape_typst("Hello #world"), "Hello \\#world");
        assert_eq!(escape_typst("*bold*"), "\\*bold\\*");
    }

    #[test]
    fn test_book_to_typst() {
        let encoder = TypstPdfEncoder::new();
        let book = Book::new("Test Book", "en");
        let typst = encoder.book_to_typst(&book);

        assert!(typst.contains("Test Book"));
        assert!(typst.contains("#set page"));
    }
}
