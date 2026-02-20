# Bookle

Ebook format conversion CLI.

## Project Structure

Rust workspace with two crates:

- `bookle-core/` - Core library: decoders, encoders, IR types
- `bookle-cli/` - CLI tool (clap)

## Commands

```bash
cargo build                    # Build all
cargo build --release          # Release build
cargo test --workspace         # Run all tests
cargo test -p bookle-core      # Test specific crate
cargo clippy --workspace       # Lint
cargo fmt --all                # Format
```

### CLI Usage

```bash
cargo run -p bookle-cli -- convert input.epub -o output.typ
cargo run -p bookle-cli -- info input.epub --json
cargo run -p bookle-cli -- validate input.epub
cargo run -p bookle-cli -- batch ./ebooks -o ./out -f epub
```

## Architecture

### Intermediate Representation (IR)

All formats convert through a semantic IR:

```
Book
├── Metadata (title, authors, language, cover, etc.)
├── TableOfContents
├── Chapters[]
│   └── Blocks[] (Header, Paragraph, List, CodeBlock, Blockquote, Image, Table)
└── Resources (images, fonts, stylesheets)
```

### Supported Formats

**Decoders (input):** EPUB, Markdown, PDF, MOBI/AZW
**Encoders (output):** EPUB 3, Typst (PDF via typst)

## Code Conventions

- Rust 2021 edition
- Error handling: `thiserror` for library errors, `anyhow` for applications
- Async runtime: `tokio`
- Logging: `tracing`
- Serialization: `serde` + `serde_json`

## CI/CD

GitHub Actions builds CLI binaries on release creation (`.github/workflows/release.yml`):

**Targets:** Linux (x86_64), macOS (x86_64, aarch64), Windows (x86_64)

To create a release:
```bash
git tag v0.1.0
git push origin v0.1.0
# Then create release on GitHub from the tag
```
