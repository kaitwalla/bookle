//! Error types for Bookle Core

use thiserror::Error;

/// Result type alias using BookleError
pub type Result<T> = std::result::Result<T, BookleError>;

/// Top-level error type for all Bookle operations
#[derive(Debug, Error)]
pub enum BookleError {
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("Conversion error: {0}")]
    Conversion(#[from] ConversionError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors that occur during parsing of input formats
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid HTML: {0}")]
    InvalidHtml(String),

    #[error("Invalid EPUB: {0}")]
    InvalidEpub(String),

    #[error("Invalid MOBI: {0}")]
    InvalidMobi(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Malformed content: {0}")]
    MalformedContent(String),
}

/// Errors that occur during encoding/conversion
#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Encoding failed: {0}")]
    EncodingFailed(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Invalid template: {0}")]
    InvalidTemplate(String),

    #[error("Typst compilation error: {0}")]
    TypstError(String),
}

/// Errors that occur during storage operations
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("Presigned URL not supported by this backend")]
    PresignedUrlNotSupported,
}
