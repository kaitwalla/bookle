# bookle-desktop

Cross-platform desktop application for Bookle using Tauri v2.

## Features

- Native file dialogs for import/export
- Local library storage
- Menu bar with keyboard shortcuts
- Cross-platform (macOS, Windows, Linux)

## Development

### Prerequisites

- Rust 1.70+
- Node.js 18+
- Platform-specific dependencies for Tauri

### Setup

```bash
cd bookle-desktop
npm install
```

### Running

```bash
# Development mode with hot reload
npm run tauri dev

# Build for production
npm run tauri build
```

## Architecture

```
bookle-desktop/
├── src/               # Frontend (TypeScript/HTML)
│   ├── main.ts        # Entry point
│   └── styles.css
├── src-tauri/         # Tauri backend (Rust)
│   ├── src/
│   │   └── lib.rs     # Commands and app setup
│   ├── Cargo.toml
│   └── tauri.conf.json
└── package.json
```

## Tauri Commands

### Library Management

```typescript
// List all books
const books = await invoke('list_books');

// Import a book
const book = await invoke('import_book', { path: '/path/to/book.epub' });

// Get book details
const details = await invoke('get_book', { id: 'uuid' });

// Delete a book
await invoke('delete_book', { id: 'uuid' });

// Export a book
await invoke('export_book', {
  id: 'uuid',
  format: 'epub',
  outputPath: '/path/to/output.epub'
});
```

### File Dialogs

```typescript
// Open file dialog
const path = await invoke('open_file_dialog');
if (path) {
  await invoke('import_book', { path });
}

// Save file dialog
const savePath = await invoke('save_file_dialog', {
  defaultName: 'book.epub',
  format: 'epub'
});
```

### Utility

```typescript
// Get app version
const version = await invoke('get_version');
```

## Menu Bar

| Menu | Item | Shortcut |
|------|------|----------|
| File | Import Book... | Cmd/Ctrl+O |
| File | Export Book... | Cmd/Ctrl+Shift+E |
| File | Quit | Cmd/Ctrl+Q |
| Edit | Undo, Redo, Cut, Copy, Paste, Select All | Standard |
| View | Refresh Library | Cmd/Ctrl+R |
| Help | About Bookle | - |

## Data Storage

Books are stored in the platform-specific app data directory:

- **macOS**: `~/Library/Application Support/com.bookle.Bookle/`
- **Windows**: `%APPDATA%\bookle\Bookle\`
- **Linux**: `~/.local/share/bookle/`

```
data/
├── books/           # Book IR JSON files
├── cache/           # Cached conversions
└── library.json     # Library index
```

## Configuration

Edit `src-tauri/tauri.conf.json`:

```json
{
  "productName": "Bookle",
  "version": "0.1.0",
  "identifier": "com.bookle.app",
  "app": {
    "windows": [{
      "title": "Bookle",
      "width": 1200,
      "height": 800
    }]
  }
}
```

## Building

### macOS

```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/dmg/
```

### Windows

```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/msi/
```

### Linux

```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/deb/
```

## License

MIT
