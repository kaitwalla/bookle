<p align="center">
  <img src="bookle-logo.png" alt="Bookle" width="128">
</p>

# Bookle

A command-line ebook format converter.

## Supported Formats

### Input Formats (Decoders)
| Format | Extensions |
|--------|------------|
| EPUB | .epub |
| Markdown | .md, .markdown |
| PDF | .pdf |
| MOBI/AZW | .mobi, .azw, .azw3, .prc |

### Output Formats (Encoders)
| Format | Extensions |
|--------|------------|
| EPUB 3 | .epub |
| Typst | .typ |

## Installation

### From Source

```bash
git clone https://github.com/kaitwalla/bookle.git
cd bookle
cargo install --path bookle-cli
```

### From Releases

Download pre-built binaries from [GitHub Releases](https://github.com/kaitwalla/bookle/releases).

## Usage

```bash
# Convert a single file
bookle convert input.epub -o output.typ

# Display book information
bookle info input.epub --json

# Validate an ebook
bookle validate input.epub --strict

# Batch convert a directory
bookle batch ./ebooks -o ./converted -f epub -j 4
```

## Project Structure

```
bookle/
├── bookle-core/       # Core library (Rust)
│   ├── src/
│   │   ├── decoder/   # Format decoders (EPUB, Markdown, PDF, MOBI)
│   │   ├── encoder/   # Format encoders (EPUB, Typst)
│   │   ├── types/     # IR types (Book, Chapter, Block, Inline)
│   │   └── storage/   # Storage abstraction
│   └── tests/
└── bookle-cli/        # Command-line tool (Rust/Clap)
    ├── src/
    │   └── commands/  # CLI commands
    └── tests/
```

## Architecture

### Intermediate Representation (IR)

Bookle uses a semantic IR to represent ebook content, enabling lossless conversion between formats:

```
Book
├── Metadata (title, authors, language, etc.)
├── TableOfContents
├── Chapters[]
│   ├── Title
│   └── Blocks[]
│       ├── Header
│       ├── Paragraph
│       ├── List
│       ├── CodeBlock
│       ├── Blockquote
│       ├── Image
│       ├── Table
│       └── ThematicBreak
└── Resources (images, fonts)
```

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace

# Format
cargo fmt --all
```

## License

MIT License - see [LICENSE](LICENSE) for details.
