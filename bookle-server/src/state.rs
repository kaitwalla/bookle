//! Application state

use anyhow::Result;
use bookle_core::storage::{LocalStorage, StorageProvider};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Storage provider for books
    pub storage: Arc<dyn StorageProvider>,

    /// Base path for storage
    pub storage_path: PathBuf,

    /// In-memory book index (would be a database in production)
    pub library: Arc<RwLock<Library>>,

    /// Channel for SSE events
    pub event_tx: broadcast::Sender<ServerEvent>,
}

/// Library index storing book metadata
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Library {
    pub books: HashMap<String, BookEntry>,
}

/// A book entry in the library
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookEntry {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub language: String,
    pub description: Option<String>,
    pub chapters: usize,
    pub added_at: chrono::DateTime<chrono::Utc>,
}

impl Library {
    /// Load library from a JSON file
    pub async fn load(path: &std::path::Path) -> Result<Self> {
        // Read file directly, handle NotFound as empty library
        match tokio::fs::read_to_string(path).await {
            Ok(data) => Ok(serde_json::from_str(&data)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Save library to a JSON file atomically
    /// Writes to a temp file then renames to avoid partial writes
    pub async fn save(&self, path: &std::path::Path) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;

        // Write to temp file in same directory (ensures same filesystem for rename)
        let temp_path = path.with_extension("json.tmp");
        tokio::fs::write(&temp_path, &data).await?;

        // Atomic rename
        tokio::fs::rename(&temp_path, path).await?;
        Ok(())
    }
}

/// Server-sent events
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// A new book was uploaded
    BookUploaded { id: String, title: String },

    /// Book conversion completed
    ConversionComplete { id: String, format: String },

    /// An error occurred
    Error { message: String },
}

impl AppState {
    /// Create new application state
    pub async fn new() -> Result<Self> {
        // Default to local storage in current directory
        let storage_path =
            std::env::var("BOOKLE_STORAGE_PATH").unwrap_or_else(|_| "./bookle_data".to_string());
        let storage_path = PathBuf::from(storage_path);

        // Create storage directories
        tokio::fs::create_dir_all(&storage_path).await?;
        tokio::fs::create_dir_all(storage_path.join("books")).await?;
        tokio::fs::create_dir_all(storage_path.join("cache")).await?;

        // Load library index
        let library_path = storage_path.join("library.json");
        let library = match Library::load(&library_path).await {
            Ok(lib) => lib,
            Err(e) => {
                tracing::warn!("Failed to load library index, starting fresh: {}", e);
                Library::default()
            }
        };

        let storage = Arc::new(LocalStorage::new(&storage_path));
        let (event_tx, _) = broadcast::channel(100);

        Ok(Self {
            storage,
            storage_path,
            library: Arc::new(RwLock::new(library)),
            event_tx,
        })
    }

    /// Get path to library index file
    pub fn library_path(&self) -> PathBuf {
        self.storage_path.join("library.json")
    }

    /// Validate that an ID is a safe filename (UUID format)
    /// Prevents path traversal attacks
    fn validate_id(id: &str) -> Result<()> {
        // UUIDs contain only hex digits and hyphens
        if id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') && !id.is_empty() {
            Ok(())
        } else {
            anyhow::bail!("Invalid ID format")
        }
    }

    /// Get path for storing a book's IR
    /// Returns error if ID is not a valid safe identifier
    pub fn book_path(&self, id: &str) -> Result<PathBuf> {
        Self::validate_id(id)?;
        Ok(self.storage_path.join("books").join(format!("{}.json", id)))
    }

    /// Get path for cached conversions
    /// Returns error if ID or format is not a valid safe identifier
    pub fn cache_path(&self, id: &str, format: &str) -> Result<PathBuf> {
        Self::validate_id(id)?;
        // Format should be a simple extension (epub, typ, etc.)
        if !format.chars().all(|c| c.is_ascii_alphanumeric()) || format.is_empty() {
            anyhow::bail!("Invalid format");
        }
        Ok(self
            .storage_path
            .join("cache")
            .join(format!("{}.{}", id, format)))
    }

    /// Save the library index
    pub async fn save_library(&self) -> Result<()> {
        let library = self.library.read().await;
        library.save(&self.library_path()).await
    }

    /// Subscribe to server events
    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.event_tx.subscribe()
    }

    /// Broadcast an event
    pub fn broadcast(&self, event: ServerEvent) {
        // Ignore errors (no subscribers)
        let _ = self.event_tx.send(event);
    }
}
