# bookle-core

Core library for the Bookle ebook management system. Provides format decoders, encoders, and the intermediate representation (IR) types.

## Features

- **Format Decoders**: EPUB, Markdown, PDF, MOBI/AZW
- **Format Encoders**: EPUB 3, Typst (PDF source)
- **Semantic IR**: Lossless representation of ebook content
- **Storage Abstraction**: Local filesystem with OpenDAL support

## Usage

```rust
use bookle_core::decoder::decoder_for_extension;
use bookle_core::encoder::encoder_for_format;
use std::fs::File;
use std::io::BufReader;

// Decode an EPUB file
let file = File::open("book.epub")?;
let mut reader = BufReader::new(file);
let decoder = decoder_for_extension("epub").unwrap();
let book = decoder.decode(&mut reader)?;

println!("Title: {}", book.metadata.title);
println!("Chapters: {}", book.chapters.len());

// Encode to Typst
let encoder = encoder_for_format("typ").unwrap();
let mut output = Vec::new();
encoder.encode(&book, &mut output)?;
```

## Supported Formats

### Decoders

| Format | Extensions | Features |
|--------|------------|----------|
| EPUB 2/3 | .epub | Full metadata, TOC, chapters, images |
| Markdown | .md, .markdown | CommonMark + tables, footnotes |
| PDF | .pdf | Text extraction with heading detection |
| MOBI/AZW | .mobi, .azw, .azw3, .prc | Metadata, HTML content |

### Encoders

| Format | Extensions | Features |
|--------|------------|----------|
| EPUB 3 | .epub | Full IR support, embedded resources |
| Typst | .typ | Configurable page size, margins |

## IR Types

### Book Structure

```rust
pub struct Book {
    pub id: Uuid,
    pub metadata: Metadata,
    pub toc: Vec<TocEntry>,
    pub chapters: Vec<Chapter>,
    pub resources: ResourceStore,
}

pub struct Chapter {
    pub id: String,
    pub title: String,
    pub content: Vec<Block>,
}
```

### Block Types

```rust
pub enum Block {
    Header { level: u8, content: Vec<Inline>, anchor: Option<String> },
    Paragraph(Vec<Inline>),
    List { items: Vec<Vec<Block>>, ordered: bool },
    Image { resource_key: String, caption: Option<String>, alt: String },
    CodeBlock { lang: Option<String>, code: String },
    Blockquote(Vec<Block>),
    ThematicBreak,
    Table(TableData),
    Footnote { id: String, content: Vec<Block> },
}
```

### Inline Types

```rust
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Code(String),
    Link { children: Vec<Inline>, url: String },
    Superscript(Vec<Inline>),
    Subscript(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    FootnoteRef { id: String },
    Ruby { base: String, annotation: String },
    Break,
}
```

## Testing

```bash
cargo test -p bookle-core

# Run with snapshots
cargo test -p bookle-core -- --nocapture
```

## Dependencies

- `epub` / `epub-builder`: EPUB handling
- `pulldown-cmark`: Markdown parsing
- `pdf-extract`: PDF text extraction
- `mobi`: MOBI/AZW parsing
- `scraper`: HTML parsing
- `serde`: Serialization

## License

MIT
