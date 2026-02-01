# Bookle

Ebook management and format conversion platform.

## Project Structure

Rust monorepo with Cargo workspace:

- `bookle-core/` - Core library: decoders, encoders, IR types
- `bookle-cli/` - CLI tool (clap)
- `bookle-server/` - REST API (axum)
- `bookle-desktop/` - Desktop app (Tauri v2)
- `frontend-web/` - Web UI (React + Vite + Tailwind v4)

## Commands

### Rust (all crates)
```bash
cargo build                    # Build all
cargo build --release          # Release build
cargo test --workspace         # Run all tests
cargo test -p bookle-core      # Test specific crate
cargo clippy --workspace       # Lint
cargo fmt --all                # Format
```

### Server
```bash
cargo run -p bookle-server                           # Dev server (localhost:3000)
BOOKLE_STORAGE_PATH=/path cargo run -p bookle-server # Custom storage
```

### CLI
```bash
cargo run -p bookle-cli -- convert input.epub -o output.typ
cargo run -p bookle-cli -- info input.epub --json
cargo run -p bookle-cli -- validate input.epub
cargo run -p bookle-cli -- batch ./ebooks -o ./out -f epub
```

### Frontend Web
```bash
cd frontend-web
npm install
npm run dev      # Dev server (localhost:5173)
npm run build    # Production build
npm run lint     # ESLint
```

### Desktop App
```bash
cd bookle-desktop
npm install
npm run tauri dev    # Dev mode
npm run tauri build  # Production build
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

### API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /health | Health check |
| GET | /api/v1/library | List books |
| POST | /api/v1/library | Upload book |
| GET | /api/v1/library/:id | Get book |
| DELETE | /api/v1/library/:id | Delete book |
| GET | /api/v1/library/:id/download?format= | Download/convert |
| GET | /api/v1/sync | SSE stream |

## Code Conventions

- Rust 2021 edition
- Error handling: `thiserror` for library errors, `anyhow` for applications
- Async runtime: `tokio`
- Logging: `tracing`
- Serialization: `serde` + `serde_json`
- Frontend: React 19, TypeScript, Tailwind CSS v4, React Query

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| BOOKLE_STORAGE_PATH | ./bookle_data | Book storage directory |
| BOOKLE_CORS_ORIGINS | localhost origins | Allowed CORS origins |
| RUST_LOG | bookle_server=debug | Log level filter |

## CI/CD

### Release Workflow

GitHub Actions automatically builds on release creation (`.github/workflows/release.yml`):

**CLI builds:** Linux (x86_64), macOS (x86_64, aarch64), Windows (x86_64)
**Desktop builds:** Linux, macOS (universal), Windows

To create a release:
```bash
git tag v0.1.0
git push origin v0.1.0
# Then create release on GitHub from the tag
```
