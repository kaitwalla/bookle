//! Resource management for embedded assets (images, fonts, etc.)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// How resource data is stored
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "storage", rename_all = "snake_case")]
pub enum ResourceData {
    /// Data stored directly in memory
    Inline(#[serde(with = "base64_serde")] Vec<u8>),

    /// Data stored in a temporary file
    TempFile { path: PathBuf },

    /// Data stored in external storage backend
    External { backend: String, path: String },
}

impl ResourceData {
    /// Create inline resource data
    pub fn inline(data: Vec<u8>) -> Self {
        ResourceData::Inline(data)
    }

    /// Get data as bytes (loads from temp file if needed)
    pub fn as_bytes(&self) -> std::io::Result<Vec<u8>> {
        match self {
            ResourceData::Inline(data) => Ok(data.clone()),
            ResourceData::TempFile { path } => std::fs::read(path),
            ResourceData::External { .. } => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Cannot read external resource synchronously",
                ))
            }
        }
    }
}

/// A single resource (image, font, stylesheet, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    /// MIME type (e.g., "image/png", "font/woff2")
    pub mime_type: String,

    /// The resource data
    pub data: ResourceData,

    /// Original filename (optional, for reference)
    pub original_filename: Option<String>,
}

impl Resource {
    /// Create a new inline resource
    pub fn new(mime_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            mime_type: mime_type.into(),
            data: ResourceData::Inline(data),
            original_filename: None,
        }
    }

    /// Set the original filename
    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.original_filename = Some(filename.into());
        self
    }
}

/// Content-addressed resource store
/// Keys are SHA-256 hashes of the resource data for deduplication
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ResourceStore {
    resources: HashMap<String, Resource>,
}

impl ResourceStore {
    /// Create an empty resource store
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a resource, returning its content-addressed key (SHA-256 hash)
    pub fn add(&mut self, resource: Resource) -> String {
        use sha2::{Sha256, Digest};

        // For inline data, compute SHA-256 hash
        // For other storage types, use a UUID
        let key = match &resource.data {
            ResourceData::Inline(data) => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                let result = hasher.finalize();
                hex::encode(result)
            }
            _ => uuid::Uuid::new_v4().to_string(),
        };

        self.resources.insert(key.clone(), resource);
        key
    }

    /// Get a resource by key
    pub fn get(&self, key: &str) -> Option<&Resource> {
        self.resources.get(key)
    }

    /// Remove a resource by key
    pub fn remove(&mut self, key: &str) -> Option<Resource> {
        self.resources.remove(key)
    }

    /// Iterate over all resources
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Resource)> {
        self.resources.iter()
    }

    /// Number of resources in the store
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
}

/// Base64 serialization for binary data
mod base64_serde {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(data))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}
