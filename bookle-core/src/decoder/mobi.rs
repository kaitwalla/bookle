//! MOBI/AZW decoder implementation

use crate::error::ParseError;
use crate::types::{Block, Book, Chapter, Inline, Metadata};
use mobi::Mobi;
use std::io::Read;

/// Decoder for MOBI/AZW format
///
/// Supports MOBI (Mobipocket) and AZW (Amazon Kindle) formats.
pub struct MobiDecoder {
    /// Whether to sanitize HTML strictly
    strict_sanitization: bool,
}

impl MobiDecoder {
    pub fn new() -> Self {
        Self {
            strict_sanitization: true,
        }
    }

    /// Set strict sanitization mode
    pub fn with_strict_sanitization(mut self, strict: bool) -> Self {
        self.strict_sanitization = strict;
        self
    }

    /// Parse HTML content into Block AST (reusing logic from EPUB decoder approach)
    fn parse_html_to_blocks(&self, html: &str) -> Result<Vec<Block>, ParseError> {
        use scraper::{Html, Selector};

        let document = Html::parse_document(html);

        // Select body content (or root if no body)
        let body_selector = Selector::parse("body").unwrap();
        let root = document
            .select(&body_selector)
            .next()
            .map(|el| el.inner_html())
            .unwrap_or_else(|| html.to_string());

        // Re-parse the body content
        let fragment = Html::parse_fragment(&root);
        let mut blocks = Vec::new();

        // Process top-level elements
        for element in fragment.root_element().children() {
            if let Some(el) = element.value().as_element() {
                if let Some(block) = self.element_to_block(el, &fragment, element)? {
                    blocks.push(block);
                }
            }
        }

        Ok(blocks)
    }

    /// Convert an HTML element to a Block
    fn element_to_block(
        &self,
        element: &scraper::node::Element,
        _document: &scraper::Html,
        node: ego_tree::NodeRef<scraper::Node>,
    ) -> Result<Option<Block>, ParseError> {
        let tag = element.name();

        match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = tag[1..].parse::<u8>().unwrap_or(1);
                let content = self.children_to_inlines(node)?;
                let anchor = element.attr("id").map(|s| s.to_string());
                Ok(Some(Block::Header {
                    level,
                    content,
                    anchor,
                }))
            }
            "p" => {
                let content = self.children_to_inlines(node)?;
                if content.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(Block::Paragraph(content)))
                }
            }
            "ul" | "ol" => {
                let ordered = tag == "ol";
                let mut items = Vec::new();

                for child in node.children() {
                    if let Some(el) = child.value().as_element() {
                        if el.name() == "li" {
                            let item_blocks = self.li_to_blocks(child)?;
                            items.push(item_blocks);
                        }
                    }
                }

                Ok(Some(Block::List { items, ordered }))
            }
            "blockquote" => {
                let inner_blocks = self.children_to_blocks(node)?;
                Ok(Some(Block::Blockquote(inner_blocks)))
            }
            "pre" => {
                let code = self.get_text_content(node);
                let lang = None;
                Ok(Some(Block::CodeBlock { lang, code }))
            }
            "hr" => Ok(Some(Block::ThematicBreak)),
            "div" | "section" | "article" => {
                // Container elements - recurse into children
                let inner = self.children_to_blocks(node)?;
                if inner.len() == 1 {
                    Ok(Some(inner.into_iter().next().unwrap()))
                } else if inner.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(Block::Blockquote(inner)))
                }
            }
            "img" => {
                let src = element.attr("src").unwrap_or_default().to_string();
                let alt = element.attr("alt").unwrap_or_default().to_string();
                Ok(Some(Block::Image {
                    resource_key: src,
                    caption: None,
                    alt,
                }))
            }
            _ => {
                // Unknown block element - try to extract as paragraph
                let content = self.children_to_inlines(node)?;
                if content.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(Block::Paragraph(content)))
                }
            }
        }
    }

    /// Convert children to blocks
    fn children_to_blocks(
        &self,
        node: ego_tree::NodeRef<scraper::Node>,
    ) -> Result<Vec<Block>, ParseError> {
        let mut blocks = Vec::new();
        for child in node.children() {
            if let Some(el) = child.value().as_element() {
                if let Some(block) =
                    self.element_to_block(el, &scraper::Html::new_document(), child)?
                {
                    blocks.push(block);
                }
            }
        }
        Ok(blocks)
    }

    /// Convert list item to blocks
    fn li_to_blocks(
        &self,
        node: ego_tree::NodeRef<scraper::Node>,
    ) -> Result<Vec<Block>, ParseError> {
        let has_blocks = node.children().any(|c| {
            c.value()
                .as_element()
                .map(|e| matches!(e.name(), "p" | "ul" | "ol" | "blockquote" | "pre"))
                .unwrap_or(false)
        });

        if has_blocks {
            self.children_to_blocks(node)
        } else {
            let inlines = self.children_to_inlines(node)?;
            if inlines.is_empty() {
                Ok(vec![])
            } else {
                Ok(vec![Block::Paragraph(inlines)])
            }
        }
    }

    /// Convert children to inline elements
    fn children_to_inlines(
        &self,
        node: ego_tree::NodeRef<scraper::Node>,
    ) -> Result<Vec<Inline>, ParseError> {
        let mut inlines = Vec::new();

        for child in node.children() {
            match child.value() {
                scraper::Node::Text(text) => {
                    let s = text.trim();
                    if !s.is_empty() {
                        inlines.push(Inline::Text(s.to_string()));
                    }
                }
                scraper::Node::Element(el) => {
                    let inline = self.element_to_inline(el, child)?;
                    inlines.extend(inline);
                }
                _ => {}
            }
        }

        Ok(inlines)
    }

    /// Convert an HTML element to inline elements
    fn element_to_inline(
        &self,
        element: &scraper::node::Element,
        node: ego_tree::NodeRef<scraper::Node>,
    ) -> Result<Vec<Inline>, ParseError> {
        let tag = element.name();

        match tag {
            "b" | "strong" => {
                let children = self.children_to_inlines(node)?;
                Ok(vec![Inline::Bold(children)])
            }
            "i" | "em" => {
                let children = self.children_to_inlines(node)?;
                Ok(vec![Inline::Italic(children)])
            }
            "code" => {
                let text = self.get_text_content(node);
                Ok(vec![Inline::Code(text)])
            }
            "a" => {
                let children = self.children_to_inlines(node)?;
                let url = element.attr("href").unwrap_or("#").to_string();
                Ok(vec![Inline::Link { children, url }])
            }
            "sup" => {
                let children = self.children_to_inlines(node)?;
                Ok(vec![Inline::Superscript(children)])
            }
            "sub" => {
                let children = self.children_to_inlines(node)?;
                Ok(vec![Inline::Subscript(children)])
            }
            "s" | "strike" | "del" => {
                let children = self.children_to_inlines(node)?;
                Ok(vec![Inline::Strikethrough(children)])
            }
            "br" => Ok(vec![Inline::Break]),
            "span" => self.children_to_inlines(node),
            _ => {
                let text = self.get_text_content(node);
                if text.is_empty() {
                    Ok(vec![])
                } else {
                    Ok(vec![Inline::Text(text)])
                }
            }
        }
    }

    /// Get text content of a node
    fn get_text_content(&self, node: ego_tree::NodeRef<scraper::Node>) -> String {
        let mut text = String::new();
        for descendant in node.descendants() {
            if let scraper::Node::Text(t) = descendant.value() {
                text.push_str(t);
            }
        }
        text
    }

    /// Split content into chapters based on headers
    fn split_into_chapters(blocks: Vec<Block>) -> Vec<Chapter> {
        let mut chapters = Vec::new();
        let mut current_blocks = Vec::new();
        let mut current_title: Option<String> = None;

        for block in blocks {
            let is_chapter_heading = matches!(&block, Block::Header { level, .. } if *level <= 2);

            if is_chapter_heading {
                if !current_blocks.is_empty() || current_title.is_some() {
                    let title = current_title.take().unwrap_or_else(|| "Untitled".to_string());
                    chapters.push(Chapter::new(title).with_content(current_blocks));
                    current_blocks = Vec::new();
                }

                if let Block::Header { content, .. } = &block {
                    current_title = Some(inlines_to_text(content));
                }
                current_blocks.push(block);
            } else {
                current_blocks.push(block);
            }
        }

        if !current_blocks.is_empty() || current_title.is_some() {
            let title = current_title.unwrap_or_else(|| "Untitled".to_string());
            chapters.push(Chapter::new(title).with_content(current_blocks));
        }

        if chapters.is_empty() {
            chapters.push(Chapter::new("Content").with_content(vec![]));
        }

        chapters
    }
}

impl Default for MobiDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Decoder for MobiDecoder {
    fn decode(&self, reader: &mut dyn Read) -> Result<Book, ParseError> {
        // Read all data into memory
        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .map_err(|e| ParseError::InvalidMobi(format!("Failed to read MOBI: {}", e)))?;

        // Parse MOBI file
        let mobi = Mobi::new(&data)
            .map_err(|e| ParseError::InvalidMobi(format!("Invalid MOBI file: {:?}", e)))?;

        // Extract metadata
        let title = mobi.title().to_string();
        let author = mobi.author().map(|s| s.to_string());
        let publisher = mobi.publisher().map(|s| s.to_string());

        let mut metadata = Metadata::new(title, "en");
        if let Some(author) = author {
            metadata.creator = vec![author];
        }
        metadata.publisher = publisher;

        let mut book = Book::with_metadata(metadata);

        // Extract content
        // The mobi crate provides content() which returns the HTML content
        let content = mobi.content_as_string_lossy();

        // Parse the HTML content
        let blocks = self.parse_html_to_blocks(&content)?;

        // Split into chapters
        let chapters = Self::split_into_chapters(blocks);
        for chapter in chapters {
            book.add_chapter(chapter);
        }

        Ok(book)
    }

    fn supported_extensions(&self) -> &[&str] {
        &["mobi", "azw", "azw3", "prc"]
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-mobipocket-ebook", "application/vnd.amazon.ebook"]
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
    fn test_parse_simple_html() {
        let decoder = MobiDecoder::new();
        let html = r#"
            <body>
                <h1>Chapter Title</h1>
                <p>This is a <strong>bold</strong> paragraph.</p>
            </body>
        "#;

        let blocks = decoder.parse_html_to_blocks(html).unwrap();
        assert_eq!(blocks.len(), 2);

        match &blocks[0] {
            Block::Header { level, content, .. } => {
                assert_eq!(*level, 1);
                assert_eq!(content.len(), 1);
            }
            _ => panic!("Expected header"),
        }
    }

    #[test]
    fn test_parse_list() {
        let decoder = MobiDecoder::new();
        let html = r#"
            <ul>
                <li>Item 1</li>
                <li>Item 2</li>
            </ul>
        "#;

        let blocks = decoder.parse_html_to_blocks(html).unwrap();
        assert_eq!(blocks.len(), 1);

        if let Block::List { items, ordered } = &blocks[0] {
            assert!(!ordered);
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected list");
        }
    }
}
