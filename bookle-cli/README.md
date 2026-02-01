# bookle-cli

Command-line interface for the Bookle ebook management system.

## Installation

```bash
# Build from source
cargo install --path bookle-cli

# Or run directly
cargo run -p bookle-cli -- <command>
```

## Commands

### Convert

Convert an ebook to another format.

```bash
bookle convert <input> -o <output> -f <format>
```

**Options:**
- `-o, --output <path>`: Output file path (required)
- `-f, --format <format>`: Output format: `epub`, `pdf` (default: epub)

**Examples:**
```bash
# EPUB to Typst
bookle convert book.epub -o book.typ -f pdf

# Markdown to EPUB
bookle convert document.md -o document.epub -f epub

# PDF to EPUB
bookle convert scanned.pdf -o extracted.epub
```

### Info

Display information about an ebook.

```bash
bookle info <input> [--json]
```

**Options:**
- `--json`: Output as JSON

**Examples:**
```bash
# Human-readable output
bookle info book.epub

# JSON output for scripting
bookle info book.epub --json | jq '.title'
```

**Output:**
```
Title: Example Book
Authors: John Doe
Language: en
Chapters: 12
```

### Validate

Validate an ebook file structure.

```bash
bookle validate <input> [--strict]
```

**Options:**
- `--strict`: Enable strict validation mode

**Examples:**
```bash
# Basic validation
bookle validate book.epub

# Strict mode
bookle validate book.epub --strict
```

### Batch

Convert multiple ebooks in parallel.

```bash
bookle batch <input_dir> -o <output_dir> -f <format> [-j <jobs>]
```

**Options:**
- `-o, --output-dir <path>`: Output directory (required)
- `-f, --format <format>`: Output format (default: epub)
- `-j, --jobs <n>`: Parallel jobs (default: 4, minimum: 1)

**Examples:**
```bash
# Convert all EPUBs to Typst
bookle batch ./ebooks -o ./converted -f pdf

# With 8 parallel jobs
bookle batch ./library -o ./output -f epub -j 8
```

## Global Options

- `-v, --verbose`: Enable verbose output
- `-h, --help`: Print help information
- `-V, --version`: Print version

## Supported Formats

### Input (Auto-detected by extension)
- EPUB (.epub)
- Markdown (.md, .markdown)
- PDF (.pdf)
- MOBI/AZW (.mobi, .azw, .azw3, .prc)

### Output
- EPUB 3 (.epub)
- Typst (.typ) - use `-f pdf`

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |

## Testing

```bash
cargo test -p bookle-cli

# Run specific test
cargo test -p bookle-cli test_convert_markdown_to_epub
```

## License

MIT
