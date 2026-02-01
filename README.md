# Bookle

A modern ebook management system with format conversion capabilities.

## Overview

Bookle is a Rust-based ebook management platform that converts between various ebook formats using an intermediate representation (IR). The project includes:

- **bookle-core**: Core library with format decoders/encoders and IR types
- **bookle-server**: REST API server for ebook management
- **bookle-cli**: Command-line interface for conversion and batch processing
- **bookle-desktop**: Cross-platform desktop application (Tauri)
- **frontend-web**: React-based web interface

## Supported Formats

### Input Formats (Decoders)
| Format | Extensions | Status |
|--------|------------|--------|
| EPUB | .epub | Full support |
| Markdown | .md, .markdown | Full support |
| PDF | .pdf | Text extraction |
| MOBI/AZW | .mobi, .azw, .azw3, .prc | Full support |

### Output Formats (Encoders)
| Format | Extensions | Status |
|--------|------------|--------|
| EPUB 3 | .epub | Full support |
| Typst | .typ | Full support |

## Quick Start

### Prerequisites

- Rust 1.70+ (for building)
- Node.js 18+ (for frontend/desktop)
- npm or pnpm

### Building

```bash
# Clone the repository
git clone https://github.com/your-org/bookle.git
cd bookle

# Build all Rust crates
cargo build --release

# Build frontend
cd frontend-web && npm install && npm run build
```

### Running the Server

```bash
# Start the API server (default: http://localhost:3000)
cargo run -p bookle-server

# With custom storage path
BOOKLE_STORAGE_PATH=/path/to/data cargo run -p bookle-server
```

### Using the CLI

```bash
# Convert a single file
bookle convert input.epub -o output.typ -f pdf

# Display book information
bookle info input.epub --json

# Validate an ebook
bookle validate input.epub --strict

# Batch convert a directory
bookle batch ./ebooks -o ./converted -f epub -j 4
```

### Running the Desktop App

```bash
cd bookle-desktop
npm install
npm run tauri dev
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
├── bookle-server/     # REST API server (Rust/Axum)
│   ├── src/
│   │   ├── handlers/  # Request handlers
│   │   ├── routes.rs  # API routes
│   │   └── state.rs   # Application state
│   └── tests/
├── bookle-cli/        # Command-line tool (Rust/Clap)
│   ├── src/
│   │   └── commands/  # CLI commands
│   └── tests/
├── bookle-desktop/    # Desktop app (Tauri v2)
│   ├── src/           # Frontend (TypeScript)
│   └── src-tauri/     # Tauri backend (Rust)
└── frontend-web/      # Web frontend (React/TypeScript)
    └── src/
        ├── components/
        ├── pages/
        ├── hooks/
        └── lib/
```

## API Reference

### REST Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /health | Health check |
| GET | /api/v1/library | List books (paginated) |
| POST | /api/v1/library | Upload a book |
| GET | /api/v1/library/:id | Get book details |
| DELETE | /api/v1/library/:id | Delete a book |
| GET | /api/v1/library/:id/download | Download/convert a book |
| GET | /api/v1/sync | Server-sent events |

### Query Parameters

**List Books**
- `page` (default: 1) - Page number
- `per_page` (default: 20) - Items per page
- `search` - Search by title or author

**Download Book**
- `format` - Output format: `epub`, `pdf` (Typst), `typ`

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| BOOKLE_STORAGE_PATH | ./bookle_data | Storage directory |
| BOOKLE_CORS_ORIGINS | localhost:3000,5173 | Allowed CORS origins |
| RUST_LOG | bookle_server=debug | Log level |

## Development

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p bookle-core
cargo test -p bookle-server
cargo test -p bookle-cli

# Run with verbose output
cargo test --workspace -- --nocapture
```

### Test Coverage

- **bookle-core**: 29 tests (unit + snapshot)
- **bookle-server**: 14 integration tests
- **bookle-cli**: 23 integration tests

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

### Block Types
- **Header**: H1-H6 with anchor support
- **Paragraph**: Contains inline elements
- **List**: Ordered/unordered with nested blocks
- **CodeBlock**: With language annotation
- **Blockquote**: Nested block container
- **Image**: With caption and alt text
- **Table**: Headers, rows, colspan/rowspan
- **ThematicBreak**: Horizontal rule

### Inline Types
- Text, Bold, Italic, Code
- Link, Superscript, Subscript
- Strikethrough, FootnoteRef
- Ruby (CJK annotation), Break

## License

MIT License - see [LICENSE](LICENSE) for details.
