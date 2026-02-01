# bookle-server

REST API server for the Bookle ebook management system.

## Features

- RESTful API for ebook management
- File upload with format auto-detection
- On-demand format conversion with caching
- Server-sent events (SSE) for real-time updates
- CORS support for web frontends

## Quick Start

```bash
# Run with default settings
cargo run -p bookle-server

# Run with custom storage path
BOOKLE_STORAGE_PATH=/path/to/data cargo run -p bookle-server

# Run with verbose logging
RUST_LOG=bookle_server=debug cargo run -p bookle-server
```

The server starts on `http://127.0.0.1:3000` by default.

## API Endpoints

### Health Check

```
GET /health
```

Response:
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

### List Books

```
GET /api/v1/library?page=1&per_page=20&search=query
```

Response:
```json
{
  "books": [
    {
      "id": "uuid",
      "title": "Book Title",
      "authors": ["Author Name"],
      "language": "en"
    }
  ],
  "total": 42,
  "page": 1,
  "per_page": 20
}
```

### Upload Book

```
POST /api/v1/library
Content-Type: multipart/form-data

file: <ebook file>
```

Supported formats: EPUB, Markdown, PDF, MOBI/AZW

Response:
```json
{
  "id": "uuid",
  "title": "Book Title",
  "message": "Book uploaded successfully"
}
```

### Get Book Details

```
GET /api/v1/library/:id
```

Response:
```json
{
  "id": "uuid",
  "title": "Book Title",
  "authors": ["Author Name"],
  "description": "Book description",
  "language": "en",
  "chapters": [
    { "title": "Chapter 1", "index": 0 }
  ]
}
```

### Delete Book

```
DELETE /api/v1/library/:id
```

Response: `204 No Content`

### Download/Convert Book

```
GET /api/v1/library/:id/download?format=epub
```

Formats: `epub`, `pdf` (Typst source), `typ`

Response: Binary file with appropriate Content-Type

### Server-Sent Events

```
GET /api/v1/sync
```

Events:
- `book_uploaded`: New book added
- `conversion_complete`: Format conversion finished
- `error`: Error occurred

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `BOOKLE_STORAGE_PATH` | `./bookle_data` | Data storage directory |
| `BOOKLE_CORS_ORIGINS` | localhost:3000,5173 | Allowed CORS origins |
| `RUST_LOG` | `bookle_server=debug` | Log level filter |

### CORS Configuration

```bash
# Allow all origins
BOOKLE_CORS_ORIGINS="*"

# Allow specific origins
BOOKLE_CORS_ORIGINS="https://app.example.com,https://admin.example.com"
```

## Storage Structure

```
bookle_data/
├── books/           # Book IR JSON files
│   └── {uuid}.json
├── cache/           # Cached conversions
│   └── {uuid}.{format}
└── library.json     # Library index
```

## Testing

```bash
cargo test -p bookle-server

# Run with logging
RUST_LOG=debug cargo test -p bookle-server -- --nocapture
```

## Dependencies

- `axum`: Web framework
- `tower-http`: HTTP middleware (CORS, tracing)
- `tokio`: Async runtime
- `bookle-core`: Core library

## License

MIT
