//! Storage abstraction layer using OpenDAL

use crate::error::StorageError;
use async_trait::async_trait;
use std::time::Duration;

/// Result type for storage operations
pub type StorageResult<T> = std::result::Result<T, StorageError>;

/// Abstract storage provider trait
/// Wraps OpenDAL backends with Bookle-specific functionality
#[async_trait]
pub trait StorageProvider: Send + Sync {
    /// Read data from the given path
    async fn read(&self, path: &str) -> StorageResult<Vec<u8>>;

    /// Write data to the given path
    async fn write(&self, path: &str, data: Vec<u8>) -> StorageResult<()>;

    /// Delete data at the given path
    async fn delete(&self, path: &str) -> StorageResult<()>;

    /// List entries under the given prefix
    async fn list(&self, prefix: &str) -> StorageResult<Vec<String>>;

    /// Check if a path exists
    async fn exists(&self, path: &str) -> StorageResult<bool>;

    /// Get the size of a file in bytes
    async fn size(&self, path: &str) -> StorageResult<u64>;

    /// Whether this backend supports presigned URLs
    fn supports_presigned_urls(&self) -> bool {
        false
    }

    /// Generate a presigned URL for reading (if supported)
    async fn presigned_read_url(&self, _path: &str, _expires: Duration) -> StorageResult<String> {
        Err(StorageError::PresignedUrlNotSupported)
    }

    /// Generate a presigned URL for writing (if supported)
    async fn presigned_write_url(&self, _path: &str, _expires: Duration) -> StorageResult<String> {
        Err(StorageError::PresignedUrlNotSupported)
    }
}

/// Local filesystem storage provider
pub struct LocalStorage {
    root: std::path::PathBuf,
}

impl LocalStorage {
    /// Create a new local storage provider with the given root directory
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Safely resolve a path, preventing path traversal attacks
    fn full_path(&self, path: &str) -> StorageResult<std::path::PathBuf> {
        use std::path::Component;

        // Normalize path components, rejecting any that escape the root
        let mut normalized = std::path::PathBuf::new();
        for component in std::path::Path::new(path).components() {
            match component {
                Component::Normal(c) => normalized.push(c),
                Component::CurDir => {} // Ignore "."
                Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                    return Err(StorageError::BackendError(
                        "Path traversal attempt detected".to_string(),
                    ));
                }
            }
        }

        Ok(self.root.join(normalized))
    }
}

#[async_trait]
impl StorageProvider for LocalStorage {
    async fn read(&self, path: &str) -> StorageResult<Vec<u8>> {
        let full_path = self.full_path(path)?;
        tokio::fs::read(full_path)
            .await
            .map_err(|e| StorageError::NotFound(e.to_string()))
    }

    async fn write(&self, path: &str, data: Vec<u8>) -> StorageResult<()> {
        let full_path = self.full_path(path)?;
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::BackendError(e.to_string()))?;
        }
        tokio::fs::write(full_path, data)
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))
    }

    async fn delete(&self, path: &str) -> StorageResult<()> {
        let full_path = self.full_path(path)?;
        tokio::fs::remove_file(full_path)
            .await
            .map_err(|e| StorageError::NotFound(e.to_string()))
    }

    async fn list(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let full_path = self.full_path(prefix)?;
        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(&full_path)
            .await
            .map_err(|e| StorageError::NotFound(e.to_string()))?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))?
        {
            if let Some(name) = entry.file_name().to_str() {
                entries.push(name.to_string());
            }
        }
        Ok(entries)
    }

    async fn exists(&self, path: &str) -> StorageResult<bool> {
        let full_path = self.full_path(path)?;
        tokio::fs::try_exists(full_path)
            .await
            .map_err(|e| StorageError::BackendError(e.to_string()))
    }

    async fn size(&self, path: &str) -> StorageResult<u64> {
        let full_path = self.full_path(path)?;
        let metadata = tokio::fs::metadata(full_path)
            .await
            .map_err(|e| StorageError::NotFound(e.to_string()))?;
        Ok(metadata.len())
    }
}

/// In-memory storage provider (for testing)
#[derive(Default)]
pub struct MemoryStorage {
    data: std::sync::RwLock<std::collections::HashMap<String, Vec<u8>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl StorageProvider for MemoryStorage {
    async fn read(&self, path: &str) -> StorageResult<Vec<u8>> {
        self.data
            .read()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| StorageError::NotFound(path.to_string()))
    }

    async fn write(&self, path: &str, data: Vec<u8>) -> StorageResult<()> {
        self.data.write().unwrap().insert(path.to_string(), data);
        Ok(())
    }

    async fn delete(&self, path: &str) -> StorageResult<()> {
        self.data
            .write()
            .unwrap()
            .remove(path)
            .ok_or_else(|| StorageError::NotFound(path.to_string()))?;
        Ok(())
    }

    async fn list(&self, prefix: &str) -> StorageResult<Vec<String>> {
        Ok(self
            .data
            .read()
            .unwrap()
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect())
    }

    async fn exists(&self, path: &str) -> StorageResult<bool> {
        Ok(self.data.read().unwrap().contains_key(path))
    }

    async fn size(&self, path: &str) -> StorageResult<u64> {
        self.data
            .read()
            .unwrap()
            .get(path)
            .map(|d| d.len() as u64)
            .ok_or_else(|| StorageError::NotFound(path.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryStorage::new();

        // Write
        storage.write("test.txt", b"hello".to_vec()).await.unwrap();

        // Read
        let data = storage.read("test.txt").await.unwrap();
        assert_eq!(data, b"hello");

        // Exists
        assert!(storage.exists("test.txt").await.unwrap());
        assert!(!storage.exists("missing.txt").await.unwrap());

        // Size
        assert_eq!(storage.size("test.txt").await.unwrap(), 5);

        // Delete
        storage.delete("test.txt").await.unwrap();
        assert!(!storage.exists("test.txt").await.unwrap());
    }
}
