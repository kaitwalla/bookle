//! EPUB decoder implementation

use crate::error::ParseError;
use crate::types::{Block, Book, Chapter, Inline, Metadata, TocEntry};
use std::io::Read;

/// Decoder for EPUB 2/3 format
pub struct EpubDecoder {
    /// Whether to sanitize HTML strictly
    strict_sanitization: bool,
}

impl EpubDecoder {
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

    /// Parse HTML content into Block AST
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
                let lang = None; // Could extract from class
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
                    // Wrap multiple blocks in a blockquote for now
                    // TODO: Better handling of div containers
                    Ok(Some(Block::Blockquote(inner)))
                }
            }
            "img" => {
                let src = element.attr("src").unwrap_or_default().to_string();
                let alt = element.attr("alt").unwrap_or_default().to_string();
                Ok(Some(Block::Image {
                    resource_key: src, // Will be resolved later
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
        // Check if li contains block elements
        let has_blocks = node.children().any(|c| {
            c.value()
                .as_element()
                .map(|e| matches!(e.name(), "p" | "ul" | "ol" | "blockquote" | "pre"))
                .unwrap_or(false)
        });

        if has_blocks {
            self.children_to_blocks(node)
        } else {
            // Treat as inline content wrapped in paragraph
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
            "span" => {
                // Pass through span content
                self.children_to_inlines(node)
            }
            _ => {
                // Unknown inline - extract text
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

    /// Extract metadata from EPUB
    fn extract_metadata(&self, epub: &epub::doc::EpubDoc<std::io::Cursor<Vec<u8>>>) -> Metadata {
        // Helper to get metadata value as string
        // mdata() returns Option<&MetadataItem>, we need to extract .value
        let get_meta =
            |key: &str| -> Option<String> { epub.mdata(key).map(|item| item.value.clone()) };

        // Get title
        let title = get_meta("title").unwrap_or_else(|| "Unknown Title".to_string());

        // Get language
        let language = get_meta("language").unwrap_or_else(|| "en".to_string());

        // Helper to get all metadata values for a key
        let get_meta_all = |key: &str| -> Vec<String> {
            epub.metadata
                .iter()
                .filter(|item| item.property == key)
                .map(|item| item.value.clone())
                .collect()
        };

        // Get creator(s)/author(s)
        let creators = get_meta_all("creator");

        // Get other metadata
        let description = get_meta("description");
        let publisher = get_meta("publisher");
        let identifier = get_meta("identifier").unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let subjects = get_meta_all("subject");
        let rights = get_meta("rights");

        // Build metadata
        let mut metadata = Metadata::new(title, language);
        metadata.creator = creators;
        metadata.description = description;
        metadata.publisher = publisher;
        metadata.identifier = identifier;
        metadata.subject = subjects;
        metadata.rights = rights;

        // Try to find cover image
        if let Some(cover_id) = get_meta("cover") {
            metadata.cover_resource_key = Some(cover_id);
        }

        metadata
    }

    /// Extract TOC from EPUB
    fn extract_toc(&self, epub: &epub::doc::EpubDoc<std::io::Cursor<Vec<u8>>>) -> Vec<TocEntry> {
        epub.toc
            .iter()
            .map(|nav| self.nav_point_to_toc_entry(nav, 0))
            .collect()
    }

    /// Convert EPUB NavPoint to TocEntry
    fn nav_point_to_toc_entry(&self, nav: &epub::doc::NavPoint, level: u32) -> TocEntry {
        // Convert PathBuf to String for href
        let href = nav.content.to_string_lossy().to_string();
        let mut entry = TocEntry::new(&nav.label, href).with_level(level);

        // Process children recursively
        let children: Vec<TocEntry> = nav
            .children
            .iter()
            .map(|child| self.nav_point_to_toc_entry(child, level + 1))
            .collect();

        if !children.is_empty() {
            entry = entry.with_children(children);
        }

        entry
    }

    /// Extract resources (images, fonts) from EPUB
    /// Returns (ResourceStore, mapping from original IDs to content-addressed keys)
    fn extract_resources(
        &self,
        epub: &mut epub::doc::EpubDoc<std::io::Cursor<Vec<u8>>>,
    ) -> (
        crate::types::ResourceStore,
        std::collections::HashMap<String, String>,
    ) {
        use crate::types::{Resource, ResourceStore};

        let mut store = ResourceStore::new();
        let mut id_to_key = std::collections::HashMap::new();

        // Get all resource IDs
        let resource_ids: Vec<String> = epub.resources.keys().cloned().collect();

        for id in resource_ids {
            if let Some((data, mime)) = epub.get_resource(&id) {
                // Skip HTML/XHTML content (those are chapters)
                if mime.contains("html") || mime.contains("xml") {
                    continue;
                }

                // Add resource to store
                let resource = Resource::new(&mime, data).with_filename(&id);
                let key = store.add(resource);

                // Map original ID to content-addressed key for later reference
                id_to_key.insert(id, key);
            }
        }

        (store, id_to_key)
    }

    /// Rewrite image references in blocks to use content-addressed keys
    fn rewrite_image_refs(
        blocks: &mut [Block],
        id_to_key: &std::collections::HashMap<String, String>,
    ) {
        for block in blocks {
            match block {
                Block::Image { resource_key, .. } => {
                    // Try to find the key by matching the end of the path
                    if let Some(new_key) = id_to_key
                        .iter()
                        .find(|(id, _)| {
                            resource_key.ends_with(id.as_str())
                                || id.ends_with(resource_key.as_str())
                        })
                        .map(|(_, key)| key.clone())
                    {
                        *resource_key = new_key;
                    }
                }
                Block::List { items, .. } => {
                    for item in items {
                        Self::rewrite_image_refs(item, id_to_key);
                    }
                }
                Block::Blockquote(inner) | Block::Footnote { content: inner, .. } => {
                    Self::rewrite_image_refs(inner, id_to_key);
                }
                _ => {}
            }
        }
    }

    /// Flatten TOC tree to (href, title) pairs
    fn flatten_toc(entry: &TocEntry) -> Vec<(String, String)> {
        let mut result = vec![(entry.href.clone(), entry.title.clone())];
        for child in &entry.children {
            result.extend(Self::flatten_toc(child));
        }
        result
    }

    /// Convert inline elements to plain text
    fn inlines_to_plain_text(inlines: &[Inline]) -> String {
        inlines
            .iter()
            .map(|i| match i {
                Inline::Text(s) => s.clone(),
                Inline::Bold(children)
                | Inline::Italic(children)
                | Inline::Superscript(children)
                | Inline::Subscript(children)
                | Inline::Strikethrough(children) => Self::inlines_to_plain_text(children),
                Inline::Link { children, .. } => Self::inlines_to_plain_text(children),
                Inline::Code(s) => s.clone(),
                Inline::FootnoteRef { id } => format!("[{}]", id),
                Inline::Ruby { base, .. } => base.clone(),
                Inline::Break => " ".to_string(),
            })
            .collect()
    }
}

impl Default for EpubDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Decoder for EpubDecoder {
    fn decode(&self, reader: &mut dyn Read) -> Result<Book, ParseError> {
        // Read all data into memory
        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .map_err(|e| ParseError::InvalidEpub(e.to_string()))?;

        // Parse EPUB
        let cursor = std::io::Cursor::new(data);
        let mut epub = epub::doc::EpubDoc::from_reader(cursor)
            .map_err(|e| ParseError::InvalidEpub(e.to_string()))?;

        // Extract metadata
        let metadata = self.extract_metadata(&epub);
        let mut book = Book::with_metadata(metadata);

        // Extract TOC
        book.toc = self.extract_toc(&epub);

        // Extract resources (images, fonts, etc.) with ID mapping
        let (resources, id_to_key) = self.extract_resources(&mut epub);
        book.resources = resources;

        // Build a map of TOC entries by href for chapter title lookup
        let toc_titles: std::collections::HashMap<String, String> =
            book.toc.iter().flat_map(|e| Self::flatten_toc(e)).collect();

        // Process spine (reading order)
        let spine = epub.spine.clone();
        for item in &spine {
            let item_id = &item.idref;
            if let Some((content, _mime)) = epub.get_resource_str(item_id) {
                let mut blocks = self.parse_html_to_blocks(&content)?;

                // Rewrite image references to use content-addressed keys
                Self::rewrite_image_refs(&mut blocks, &id_to_key);

                // Try to get chapter title from TOC using precise matching
                let title = toc_titles
                    .iter()
                    .find(|(href, _)| {
                        // Match by exact filename stem
                        std::path::Path::new(href)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|stem| stem == item_id)
                            .unwrap_or(false)
                            || href.ends_with(item_id)
                    })
                    .map(|(_, title)| title.clone())
                    .unwrap_or_else(|| {
                        // Try to extract title from first header in content
                        blocks
                            .iter()
                            .find_map(|b| {
                                if let Block::Header { content, .. } = b {
                                    Some(Self::inlines_to_plain_text(content))
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| item_id.clone())
                    });

                let chapter = Chapter::new(title)
                    .with_id(item_id.clone())
                    .with_content(blocks);
                book.add_chapter(chapter);
            }
        }

        Ok(book)
    }

    fn supported_extensions(&self) -> &[&str] {
        &["epub"]
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/epub+zip"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_html() {
        let decoder = EpubDecoder::new();
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
}
