//! Markdown decoder implementation

use crate::error::ParseError;
use crate::types::{Block, Book, Chapter, Inline, Metadata, TableCell, TableData};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::io::Read;

/// Decoder for Markdown format
pub struct MarkdownDecoder {
    /// Whether to enable tables extension
    enable_tables: bool,
    /// Whether to enable strikethrough extension
    enable_strikethrough: bool,
    /// Whether to enable footnotes extension
    enable_footnotes: bool,
}

impl MarkdownDecoder {
    pub fn new() -> Self {
        Self {
            enable_tables: true,
            enable_strikethrough: true,
            enable_footnotes: true,
        }
    }

    /// Enable or disable tables parsing
    pub fn with_tables(mut self, enable: bool) -> Self {
        self.enable_tables = enable;
        self
    }

    /// Enable or disable strikethrough parsing
    pub fn with_strikethrough(mut self, enable: bool) -> Self {
        self.enable_strikethrough = enable;
        self
    }

    /// Enable or disable footnotes parsing
    pub fn with_footnotes(mut self, enable: bool) -> Self {
        self.enable_footnotes = enable;
        self
    }

    fn get_parser_options(&self) -> Options {
        let mut options = Options::empty();
        if self.enable_tables {
            options.insert(Options::ENABLE_TABLES);
        }
        if self.enable_strikethrough {
            options.insert(Options::ENABLE_STRIKETHROUGH);
        }
        if self.enable_footnotes {
            options.insert(Options::ENABLE_FOOTNOTES);
        }
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
        options
    }

    /// Parse markdown content into blocks
    fn parse_markdown(&self, content: &str) -> Result<Vec<Block>, ParseError> {
        let options = self.get_parser_options();
        let parser = Parser::new_ext(content, options);
        let events: Vec<Event> = parser.collect();

        let mut state = ParserState::new();
        self.process_events(&events, &mut state)?;

        Ok(state.blocks)
    }

    /// Process markdown events into blocks
    fn process_events(&self, events: &[Event], state: &mut ParserState) -> Result<(), ParseError> {
        let mut i = 0;
        while i < events.len() {
            i = self.process_event(events, i, state)?;
        }
        Ok(())
    }

    /// Process a single event, returning the next index to process
    fn process_event(
        &self,
        events: &[Event],
        start: usize,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let event = &events[start];

        match event {
            Event::Start(tag) => self.handle_start_tag(events, start, tag.clone(), state),
            Event::Text(text) => {
                state.push_text(text.to_string());
                Ok(start + 1)
            }
            Event::Code(code) => {
                state.push_inline(Inline::Code(code.to_string()));
                Ok(start + 1)
            }
            Event::SoftBreak => {
                state.push_text(" ".to_string());
                Ok(start + 1)
            }
            Event::HardBreak => {
                state.push_inline(Inline::Break);
                Ok(start + 1)
            }
            Event::Rule => {
                state.blocks.push(Block::ThematicBreak);
                Ok(start + 1)
            }
            Event::End(_) => Ok(start + 1),
            _ => Ok(start + 1),
        }
    }

    /// Handle a start tag and process until its matching end tag
    fn handle_start_tag(
        &self,
        events: &[Event],
        start: usize,
        tag: Tag,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        match tag {
            Tag::Heading { level, id, .. } => {
                self.process_heading(events, start, level, id.as_deref(), state)
            }
            Tag::Paragraph => self.process_paragraph(events, start, state),
            Tag::BlockQuote => self.process_blockquote(events, start, state),
            Tag::CodeBlock(kind) => self.process_code_block(events, start, kind, state),
            Tag::List(start_num) => self.process_list(events, start, start_num, state),
            Tag::Item => self.process_list_item(events, start, state),
            Tag::Table(_) => self.process_table(events, start, state),
            Tag::TableHead => self.process_table_head(events, start, state),
            Tag::TableRow => self.process_table_row(events, start, state),
            Tag::TableCell => self.process_table_cell(events, start, state),
            Tag::Emphasis => self.process_emphasis(events, start, state),
            Tag::Strong => self.process_strong(events, start, state),
            Tag::Strikethrough => self.process_strikethrough(events, start, state),
            Tag::Link { dest_url, .. } => {
                self.process_link(events, start, dest_url.to_string(), state)
            }
            Tag::Image {
                dest_url, title, ..
            } => self.process_image(
                events,
                start,
                dest_url.to_string(),
                title.to_string(),
                state,
            ),
            Tag::FootnoteDefinition(label) => {
                self.process_footnote_def(events, start, label.to_string(), state)
            }
            _ => Ok(start + 1),
        }
    }

    /// Find the matching end tag for a start tag
    fn find_end_tag(&self, events: &[Event], start: usize, expected_end: &TagEnd) -> usize {
        let mut depth = 0;
        for (i, event) in events.iter().enumerate().skip(start) {
            match event {
                Event::Start(_) => depth += 1,
                Event::End(end) => {
                    depth -= 1;
                    if depth == 0 && end == expected_end {
                        return i;
                    }
                }
                _ => {}
            }
        }
        events.len()
    }

    /// Process a heading
    fn process_heading(
        &self,
        events: &[Event],
        start: usize,
        level: HeadingLevel,
        id: Option<&str>,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::Heading(level));
        let inlines = self.collect_inlines(events, start + 1, end)?;

        let level_num = match level {
            HeadingLevel::H1 => 1,
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            HeadingLevel::H4 => 4,
            HeadingLevel::H5 => 5,
            HeadingLevel::H6 => 6,
        };

        state.blocks.push(Block::Header {
            level: level_num,
            content: inlines,
            anchor: id.map(|s| s.to_string()),
        });

        Ok(end + 1)
    }

    /// Process a paragraph
    fn process_paragraph(
        &self,
        events: &[Event],
        start: usize,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::Paragraph);
        let inlines = self.collect_inlines(events, start + 1, end)?;

        if !inlines.is_empty() {
            state.blocks.push(Block::Paragraph(inlines));
        }

        Ok(end + 1)
    }

    /// Process a blockquote
    fn process_blockquote(
        &self,
        events: &[Event],
        start: usize,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::BlockQuote);

        let mut inner_state = ParserState::new();
        self.process_events(&events[start + 1..end], &mut inner_state)?;

        state.blocks.push(Block::Blockquote(inner_state.blocks));

        Ok(end + 1)
    }

    /// Process a code block
    fn process_code_block(
        &self,
        events: &[Event],
        start: usize,
        kind: CodeBlockKind,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let lang = match &kind {
            CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.to_string()),
            _ => None,
        };

        let end = self.find_end_tag(events, start, &TagEnd::CodeBlock);

        // Collect text content
        let mut code = String::new();
        for event in &events[start + 1..end] {
            if let Event::Text(text) = event {
                code.push_str(text);
            }
        }

        state.blocks.push(Block::CodeBlock { lang, code });

        Ok(end + 1)
    }

    /// Process a list
    fn process_list(
        &self,
        events: &[Event],
        start: usize,
        start_num: Option<u64>,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let ordered = start_num.is_some();
        let end = self.find_end_tag(events, start, &TagEnd::List(ordered));

        // Process list items
        let mut items: Vec<Vec<Block>> = Vec::new();
        let mut i = start + 1;

        while i < end {
            if let Event::Start(Tag::Item) = &events[i] {
                let item_end = self.find_end_tag(events, i, &TagEnd::Item);

                let mut item_state = ParserState::new();
                self.process_events(&events[i + 1..item_end], &mut item_state)?;
                items.push(item_state.blocks);

                i = item_end + 1;
            } else {
                i += 1;
            }
        }

        state.blocks.push(Block::List { items, ordered });

        Ok(end + 1)
    }

    /// Process a list item (called when inside a list)
    fn process_list_item(
        &self,
        events: &[Event],
        start: usize,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::Item);

        let mut item_state = ParserState::new();
        self.process_events(&events[start + 1..end], &mut item_state)?;

        // Store blocks temporarily - they'll be collected by the parent list
        state.blocks.extend(item_state.blocks);

        Ok(end + 1)
    }

    /// Process a table
    fn process_table(
        &self,
        events: &[Event],
        start: usize,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::Table);

        state.table_headers.clear();
        state.table_rows.clear();

        // Process table contents
        self.process_events(&events[start + 1..end], state)?;

        let table = TableData {
            headers: std::mem::take(&mut state.table_headers),
            rows: std::mem::take(&mut state.table_rows),
        };

        state.blocks.push(Block::Table(table));

        Ok(end + 1)
    }

    /// Process table head
    fn process_table_head(
        &self,
        events: &[Event],
        start: usize,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::TableHead);

        state.current_row.clear();
        self.process_events(&events[start + 1..end], state)?;
        state.table_headers = std::mem::take(&mut state.current_row);

        Ok(end + 1)
    }

    /// Process table row
    fn process_table_row(
        &self,
        events: &[Event],
        start: usize,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::TableRow);

        state.current_row.clear();
        self.process_events(&events[start + 1..end], state)?;

        if !state.current_row.is_empty() {
            state
                .table_rows
                .push(std::mem::take(&mut state.current_row));
        }

        Ok(end + 1)
    }

    /// Process table cell
    fn process_table_cell(
        &self,
        events: &[Event],
        start: usize,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::TableCell);
        let inlines = self.collect_inlines(events, start + 1, end)?;

        state.current_row.push(TableCell::new(inlines));

        Ok(end + 1)
    }

    /// Process emphasis (italic)
    fn process_emphasis(
        &self,
        events: &[Event],
        start: usize,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::Emphasis);
        let inlines = self.collect_inlines(events, start + 1, end)?;

        state.push_inline(Inline::Italic(inlines));

        Ok(end + 1)
    }

    /// Process strong (bold)
    fn process_strong(
        &self,
        events: &[Event],
        start: usize,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::Strong);
        let inlines = self.collect_inlines(events, start + 1, end)?;

        state.push_inline(Inline::Bold(inlines));

        Ok(end + 1)
    }

    /// Process strikethrough
    fn process_strikethrough(
        &self,
        events: &[Event],
        start: usize,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::Strikethrough);
        let inlines = self.collect_inlines(events, start + 1, end)?;

        state.push_inline(Inline::Strikethrough(inlines));

        Ok(end + 1)
    }

    /// Process a link
    fn process_link(
        &self,
        events: &[Event],
        start: usize,
        url: String,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::Link);
        let children = self.collect_inlines(events, start + 1, end)?;

        state.push_inline(Inline::Link { children, url });

        Ok(end + 1)
    }

    /// Process an image
    fn process_image(
        &self,
        events: &[Event],
        start: usize,
        src: String,
        alt: String,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::Image);

        // Collect alt text from children if not provided
        let alt_text = if alt.is_empty() {
            let inlines = self.collect_inlines(events, start + 1, end)?;
            inlines_to_text(&inlines)
        } else {
            alt
        };

        state.blocks.push(Block::Image {
            resource_key: src,
            caption: None,
            alt: alt_text,
        });

        Ok(end + 1)
    }

    /// Process a footnote definition
    fn process_footnote_def(
        &self,
        events: &[Event],
        start: usize,
        label: String,
        state: &mut ParserState,
    ) -> Result<usize, ParseError> {
        let end = self.find_end_tag(events, start, &TagEnd::FootnoteDefinition);

        let mut inner_state = ParserState::new();
        self.process_events(&events[start + 1..end], &mut inner_state)?;

        state.blocks.push(Block::Footnote {
            id: label,
            content: inner_state.blocks,
        });

        Ok(end + 1)
    }

    /// Collect inline elements from events
    fn collect_inlines(
        &self,
        events: &[Event],
        start: usize,
        end: usize,
    ) -> Result<Vec<Inline>, ParseError> {
        let mut state = ParserState::new();
        self.process_events(&events[start..end], &mut state)?;
        Ok(state.inlines)
    }

    /// Extract title from content (first H1)
    fn extract_title(blocks: &[Block]) -> Option<String> {
        for block in blocks {
            if let Block::Header {
                level: 1, content, ..
            } = block
            {
                return Some(inlines_to_text(content));
            }
        }
        None
    }

    /// Split blocks into chapters by H1 headings
    fn split_into_chapters(blocks: Vec<Block>) -> Vec<Chapter> {
        let mut chapters = Vec::new();
        let mut current_blocks = Vec::new();
        let mut current_title: Option<String> = None;

        for block in blocks {
            if let Block::Header {
                level: 1, content, ..
            } = &block
            {
                // Save previous chapter if exists
                if !current_blocks.is_empty() || current_title.is_some() {
                    let title = current_title
                        .take()
                        .unwrap_or_else(|| "Untitled".to_string());
                    chapters.push(Chapter::new(title).with_content(current_blocks));
                    current_blocks = Vec::new();
                }
                current_title = Some(inlines_to_text(content));
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

        // If no chapters, create a single chapter
        if chapters.is_empty() {
            chapters.push(Chapter::new("Content").with_content(vec![]));
        }

        chapters
    }
}

impl Default for MarkdownDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Decoder for MarkdownDecoder {
    fn decode(&self, reader: &mut dyn Read) -> Result<Book, ParseError> {
        // Read all content
        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .map_err(|e| ParseError::MalformedContent(format!("Failed to read markdown: {}", e)))?;

        // Parse markdown
        let blocks = self.parse_markdown(&content)?;

        // Extract title from first H1
        let title = Self::extract_title(&blocks).unwrap_or_else(|| "Untitled".to_string());

        // Create metadata
        let metadata = Metadata::new(title, "en");
        let mut book = Book::with_metadata(metadata);

        // Split into chapters by H1 headings
        let chapters = Self::split_into_chapters(blocks);
        for chapter in chapters {
            book.add_chapter(chapter);
        }

        Ok(book)
    }

    fn supported_extensions(&self) -> &[&str] {
        &["md", "markdown", "mdown", "mkd"]
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/markdown", "text/x-markdown"]
    }
}

/// Parser state for tracking context during parsing
struct ParserState {
    blocks: Vec<Block>,
    inlines: Vec<Inline>,
    // Table state
    table_headers: Vec<TableCell>,
    table_rows: Vec<Vec<TableCell>>,
    current_row: Vec<TableCell>,
}

impl ParserState {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            inlines: Vec::new(),
            table_headers: Vec::new(),
            table_rows: Vec::new(),
            current_row: Vec::new(),
        }
    }

    fn push_text(&mut self, text: String) {
        if !text.is_empty() {
            self.inlines.push(Inline::Text(text));
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        self.inlines.push(inline);
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
            Inline::Superscript(children) | Inline::Subscript(children) => {
                inlines_to_text(children)
            }
            Inline::FootnoteRef { id } => format!("[{}]", id),
            Inline::Ruby { base, .. } => base.clone(),
            Inline::Break => " ".to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::Decoder;

    #[test]
    fn test_parse_simple_markdown() {
        let decoder = MarkdownDecoder::new();
        let markdown = "# Hello World\n\nThis is a paragraph.";

        let blocks = decoder.parse_markdown(markdown).unwrap();

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
    fn test_parse_formatting() {
        let decoder = MarkdownDecoder::new();
        let markdown = "This is **bold** and *italic* text.";

        let blocks = decoder.parse_markdown(markdown).unwrap();

        assert_eq!(blocks.len(), 1);
        if let Block::Paragraph(inlines) = &blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
            assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_parse_list() {
        let decoder = MarkdownDecoder::new();
        let markdown = "- Item 1\n- Item 2\n- Item 3";

        let blocks = decoder.parse_markdown(markdown).unwrap();

        assert_eq!(blocks.len(), 1);
        if let Block::List { items, ordered } = &blocks[0] {
            assert!(!ordered);
            assert_eq!(items.len(), 3);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_code_block() {
        let decoder = MarkdownDecoder::new();
        let markdown = "```rust\nfn main() {}\n```";

        let blocks = decoder.parse_markdown(markdown).unwrap();

        assert_eq!(blocks.len(), 1);
        if let Block::CodeBlock { lang, code } = &blocks[0] {
            assert_eq!(lang.as_deref(), Some("rust"));
            assert!(code.contains("fn main()"));
        } else {
            panic!("Expected code block");
        }
    }

    #[test]
    fn test_parse_link() {
        let decoder = MarkdownDecoder::new();
        let markdown = "Check out [this link](https://example.com).";

        let blocks = decoder.parse_markdown(markdown).unwrap();

        if let Block::Paragraph(inlines) = &blocks[0] {
            let has_link = inlines
                .iter()
                .any(|i| matches!(i, Inline::Link { url, .. } if url == "https://example.com"));
            assert!(has_link);
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_decode_book() {
        let decoder = MarkdownDecoder::new();
        let markdown = "# My Book\n\nIntroduction.\n\n# Chapter 1\n\nContent here.";

        let mut cursor = std::io::Cursor::new(markdown);
        let book = decoder.decode(&mut cursor).unwrap();

        assert_eq!(book.metadata.title, "My Book");
        assert_eq!(book.chapters.len(), 2);
    }

    #[test]
    fn test_parse_table() {
        let decoder = MarkdownDecoder::new();
        let markdown = "| A | B |\n|---|---|\n| 1 | 2 |";

        let blocks = decoder.parse_markdown(markdown).unwrap();

        assert_eq!(blocks.len(), 1);
        if let Block::Table(table) = &blocks[0] {
            assert_eq!(table.headers.len(), 2);
            assert_eq!(table.rows.len(), 1);
        } else {
            panic!("Expected table");
        }
    }
}
