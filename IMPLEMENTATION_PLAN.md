# Bookle Implementation Plan

## Implementation Status

### Phase 1: Scaffolding (Complete)

| Task | Status |
|------|--------|
| Cargo workspace setup | Done |
| bookle-core (IR types, decoders, encoders, storage) | Done |
| bookle-server (Axum REST API skeleton) | Done |
| bookle-cli (clap CLI skeleton) | Done |
| bookle-desktop (Tauri v2 app) | Done |
| frontend-web (React + Tailwind) | Done |
| Workspace builds successfully | Done |

### Phase 2: Core Functionality (Complete)

| Task | Status |
|------|--------|
| EPUB decoder with metadata/TOC extraction | Done |
| Typst-based PDF encoder (outputs .typ files) | Done |
| Server handlers wired to bookle-core | Done |
| Unit tests with insta snapshots (7 snapshot tests) | Done |

### Phase 3: CLI & Frontend (Complete)

| Task | Status |
|------|--------|
| CLI convert command (EPUB to PDF/EPUB) | Done |
| CLI info command (JSON output) | Done |
| CLI validate command | Done |
| CLI batch command (parallel processing) | Done |
| EPUB encoder with resource embedding | Done |
| Frontend library view with React Query | Done |
| Frontend book detail page with download | Done |
| File upload component with drag-drop & progress | Done |

### Current Test Coverage

- **bookle-core unit tests:** 9 tests (parsers, serialization, storage)
- **bookle-core snapshot tests:** 7 tests (IR JSON, Typst output variations)
- **Total:** 16 tests, all passing

### Verified Working Features

- **CLI:** `bookle-cli convert alice.epub -o alice.typ -f pdf` converts EPUB to Typst
- **CLI:** `bookle-cli info alice.epub` shows metadata (JSON with --json)
- **CLI:** Round-trip EPUB -> IR -> EPUB preserves content
- **Frontend:** Library grid with search, upload modal, book cards
- **Frontend:** Book detail with chapter list, download buttons, delete

---

## Project Overview

Bookle is a high-performance, modular ebook management ecosystem written in Rust. All inputs (EPUB, MOBI, PDF) are converted to a strictly typed Intermediate Representation (IR) before being encoded to target formats.

---

## Phase 1: Foundation (bookle-core)

### 1.1 Workspace Setup

Create Cargo workspace with members:
- `bookle-core` - Logic library (IR, parsers, conversion engine, storage abstraction)
- `bookle-server` - REST API (Axum)
- `bookle-cli` - CLI tool for batch processing
- `bookle-desktop` - Tauri v2 application
- `frontend-web` - React/Vite SPA

### 1.2 Core Types (IR)

#### Book Structure
```rust
pub struct Book {
    pub id: Uuid,
    pub metadata: Metadata,
    pub chapters: Vec<Chapter>,
    pub resources: ResourceStore,
    pub toc: Vec<TocEntry>,
}
```

#### Metadata (Dublin Core + Extensions)
```rust
pub struct Metadata {
    pub title: String,
    pub creator: Vec<String>,
    pub subject: Vec<String>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub date: Option<DateTime<Utc>>,
    pub language: String,                    // ISO 639-1
    pub identifier: String,                  // ISBN or UUID
    pub cover_resource_key: Option<String>,  // Link to cover image
    pub series: Option<SeriesInfo>,          // Series name + position
    pub reading_direction: ReadingDirection, // LTR, RTL, TTB
    pub rights: Option<String>,              // Copyright info
}

pub struct SeriesInfo {
    pub name: String,
    pub position: Option<f32>,
}

pub enum ReadingDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
}
```

#### Semantic AST (Block)
```rust
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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

#### Resource Management
```rust
pub enum ResourceData {
    Inline(Vec<u8>),
    TempFile(PathBuf),
    External { backend: String, path: String },
}

pub struct Resource {
    pub mime_type: String,
    pub data: ResourceData,
}

pub struct ResourceStore {
    resources: HashMap<String, Resource>,  // key = sha256 hash
}
```

### 1.3 Error Taxonomy
```rust
#[derive(Debug, thiserror::Error)]
pub enum BookleError {
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("Conversion error: {0}")]
    Conversion(#[from] ConversionError),
    #[error("Storage error: {0}")]
    Storage(#[from] opendal::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Invalid HTML: {0}")]
    InvalidHtml(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Encoding failed: {0}")]
    EncodingFailed(String),
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
}
```

### 1.4 Storage Abstraction

Using OpenDAL with capability detection:
```rust
#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn read(&self, path: &str) -> Result<Vec<u8>>;
    async fn write(&self, path: &str, data: Vec<u8>) -> Result<()>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;

    fn supports_presigned_urls(&self) -> bool;
    async fn presigned_url(&self, path: &str, expires: Duration) -> Option<String>;
}
```

### 1.5 Conversion Pipeline

#### Decoder Trait
```rust
pub trait Decoder: Send + Sync {
    fn decode(&self, reader: &mut dyn Read) -> Result<Book, ParseError>;
    fn supported_extensions(&self) -> &[&str];
}
```

Implementations:
- `EpubDecoder` - EPUB 2/3 support, HTML sanitization, Block AST conversion
- `MobiDecoder` - Legacy MOBI import only (deprecated format)

#### Encoder Trait
```rust
pub trait Encoder: Send + Sync {
    fn encode(&self, book: &Book, writer: &mut dyn Write) -> Result<(), ConversionError>;
    fn format_name(&self) -> &str;
}
```

Implementations:
- `EpubEncoder` - Block AST -> XHTML -> EPUB
- `TypstPdfEncoder` - Block AST -> Typst Syntax -> PDF

### 1.6 Typst PDF Generation

```rust
pub struct TypstPdfEncoder {
    template: TypstTemplate,
    fonts: FontStore,
}

pub struct TypstTemplate {
    pub page_size: PageSize,
    pub margins: Margins,
    pub base_font_size: f32,
    pub heading_styles: HeadingStyles,
    // Extensible for user customization
}
```

---

## Phase 2: Server (bookle-server)

### 2.1 REST API (Axum)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/library` | POST | Upload file (multipart), triggers background conversion |
| `/api/v1/library` | GET | List books (pagination + search) |
| `/api/v1/library/{id}` | GET | Get book metadata |
| `/api/v1/library/{id}` | DELETE | Remove book |
| `/api/v1/library/{id}/download` | GET | Download in format (?format=pdf\|epub) |
| `/api/v1/sync` | GET | SSE stream for real-time updates |

### 2.2 Background Processing

- Use `tokio` for async task spawning
- Job queue for conversion tasks
- Progress reporting via SSE

---

## Phase 3: CLI (bookle-cli)

### 3.1 Commands

```
bookle convert <input> -o <output> --format <pdf|epub>
bookle info <file>
bookle validate <file>
bookle batch <directory> --format <pdf|epub>
```

### 3.2 Features

- Progress bars with `indicatif`
- Parallel batch processing with `rayon`
- JSON output mode for scripting

---

## Phase 4: Frontend (frontend-web)

### 4.1 Stack

- React 18 + TypeScript
- Vite for bundling
- Tailwind CSS + shadcn/ui
- React Query for server state
- React Router for navigation

### 4.2 Platform Abstraction

```typescript
interface PlatformAdapter {
  openFile(): Promise<File | null>;
  saveFile(data: Blob, filename: string): Promise<void>;
  getStoragePath(): string;
  isDesktop(): boolean;
}

// Implementations:
// - WebPlatformAdapter (browser APIs)
// - TauriPlatformAdapter (Tauri v2 APIs)
```

### 4.3 Core Features

- Library grid/list view
- Book metadata editor
- Simple IR-based reader (Block -> React components)
- Upload with progress
- Format conversion UI

---

## Phase 5: Desktop (bookle-desktop)

### 5.1 Tauri v2 Integration

- Shared React frontend via `PlatformAdapter`
- Native file dialogs
- System tray integration
- Auto-updater

### 5.2 Embedded Server

Option to run `bookle-server` embedded for local-first usage.

---

## Phase 6: Type Sharing

### 6.1 TypeShare Setup

Generate TypeScript interfaces from Rust structs:
```rust
#[derive(Serialize, Deserialize, TypeShare)]
pub struct BookSummary { ... }
```

Output: `frontend-web/src/types/generated.ts`

### 6.2 Future: Mobile Types

- Swift: Codable structs for iOS
- Kotlin: Data classes for Android

---

## Deferred: Device Integration

> **Status: Future milestone**

USB device detection and "Send to Kindle" functionality is deferred due to:
- Platform-specific complexity (macOS IOKit, Windows WMI, Linux udev)
- Multiple Kindle PIDs to support
- Mount point detection challenges

When implemented:
- `DeviceDetector` trait with mock support
- `MountPointResolver` per-platform implementations
- Background polling with `rusb`
- Target: Amazon Kindle (VID: 0x1949)

---

## Testing Strategy

### Unit Tests (bookle-core)

- **Parsers:** HTML fragment -> Block AST conversion
- **Serialization:** Block round-trip (serialize/deserialize)
- **Snapshot testing:** `insta` for AST and Typst output
- **Property testing:** `proptest` for parser robustness

### Integration Tests

- Full pipeline: EPUB -> IR -> PDF
- Storage: OpenDAL memory backend
- Golden files: Real EPUBs from Project Gutenberg

### Benchmarks

- `criterion` for conversion performance
- Memory profiling for large books

### Contract Tests

- TypeShare-generated types match API responses
- JSON fixtures for client testing

---

## Dependencies

### bookle-core
```toml
[dependencies]
uuid = { version = "1", features = ["v4", "serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
opendal = "0.45"
rayon = "1"
typst = "0.11"
epub = "2"
epub-builder = "0.7"

[dev-dependencies]
insta = { version = "1", features = ["json"] }
proptest = "1"
criterion = "0.5"
```

### bookle-server
```toml
[dependencies]
bookle-core = { path = "../bookle-core" }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### bookle-cli
```toml
[dependencies]
bookle-core = { path = "../bookle-core" }
clap = { version = "4", features = ["derive"] }
indicatif = "0.17"
anyhow = "1"
```

---

## Implementation Order

1. **bookle-core types** - IR structs, error types
2. **bookle-core storage** - OpenDAL abstraction
3. **bookle-core EPUB decoder** - HTML -> Block parsing
4. **bookle-core EPUB encoder** - Block -> XHTML -> EPUB
5. **bookle-core Typst encoder** - Block -> PDF
6. **bookle-cli** - Basic convert command
7. **bookle-server** - REST API
8. **frontend-web** - React SPA
9. **bookle-desktop** - Tauri wrapper
10. **Type sharing** - TypeShare integration

---

## Success Criteria

- [ ] EPUB -> IR -> EPUB round-trip preserves content
- [ ] EPUB -> PDF produces readable, styled output
- [ ] Server handles concurrent uploads
- [ ] Frontend works identically in browser and Tauri
- [ ] All tests pass, including property tests
- [ ] Benchmarks show <1s conversion for typical ebooks
