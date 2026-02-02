//! Library management handlers

use crate::state::{AppState, BookEntry, ServerEvent};
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use bookle_core::decoder::decoder_for_extension;
use bookle_core::encoder::encoder_for_format;
use bookle_core::Book;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use uuid::Uuid;

/// Query parameters for listing books
#[derive(Debug, Deserialize)]
pub struct ListBooksQuery {
    /// Page number (1-indexed, 0 treated as 1)
    #[serde(default = "default_page")]
    pub page: u32,

    /// Items per page
    #[serde(default = "default_per_page")]
    pub per_page: u32,

    /// Search query
    pub search: Option<String>,
}

fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    20
}

/// Sanitize page number (treat 0 as 1 to prevent underflow)
fn sanitize_page(page: u32) -> u32 {
    page.max(1)
}

/// Book summary for list response
#[derive(Debug, Serialize)]
pub struct BookSummary {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub language: String,
}

impl From<&BookEntry> for BookSummary {
    fn from(entry: &BookEntry) -> Self {
        Self {
            id: entry.id.clone(),
            title: entry.title.clone(),
            authors: entry.authors.clone(),
            language: entry.language.clone(),
        }
    }
}

/// List response with pagination
#[derive(Debug, Serialize)]
pub struct ListBooksResponse {
    pub books: Vec<BookSummary>,
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
}

/// List all books
pub async fn list_books(
    State(state): State<AppState>,
    Query(query): Query<ListBooksQuery>,
) -> Json<ListBooksResponse> {
    let library = state.library.read().await;

    // Filter by search query if provided
    let mut books: Vec<BookSummary> = library
        .books
        .values()
        .filter(|entry| {
            if let Some(ref search) = query.search {
                let search_lower = search.to_lowercase();
                entry.title.to_lowercase().contains(&search_lower)
                    || entry
                        .authors
                        .iter()
                        .any(|a| a.to_lowercase().contains(&search_lower))
            } else {
                true
            }
        })
        .map(BookSummary::from)
        .collect();

    // Sort by title
    books.sort_by(|a, b| a.title.cmp(&b.title));

    let total = books.len() as u32;

    // Paginate (sanitize page to prevent underflow)
    let page = sanitize_page(query.page);
    let start = ((page - 1) * query.per_page) as usize;
    let books: Vec<BookSummary> = books
        .into_iter()
        .skip(start)
        .take(query.per_page as usize)
        .collect();

    Json(ListBooksResponse {
        books,
        total,
        page,
        per_page: query.per_page,
    })
}

/// Book metadata response
#[derive(Debug, Serialize)]
pub struct BookResponse {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub language: String,
    pub chapters: Vec<ChapterSummary>,
}

#[derive(Debug, Serialize)]
pub struct ChapterSummary {
    pub title: String,
    pub index: usize,
}

/// Get a single book's metadata
pub async fn get_book(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<BookResponse>, StatusCode> {
    // Validate UUID
    Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Load book from storage
    let book_path = state.book_path(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let book_data = tokio::fs::read_to_string(&book_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let book: Book =
        serde_json::from_str(&book_data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let chapters: Vec<ChapterSummary> = book
        .chapters
        .iter()
        .enumerate()
        .map(|(i, c)| ChapterSummary {
            title: c.title.clone(),
            index: i,
        })
        .collect();

    Ok(Json(BookResponse {
        id: book.id.to_string(),
        title: book.metadata.title,
        authors: book.metadata.creator,
        description: book.metadata.description,
        language: book.metadata.language,
        chapters,
    }))
}

/// Upload response
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub title: String,
    pub message: String,
}

/// Upload a new book
pub async fn upload_book(
    State(state): State<AppState>,
    mut multipart: axum_extra::extract::Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();

        if name == "file" {
            let filename = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            // Get file extension
            let extension = std::path::Path::new(&filename)
                .extension()
                .and_then(|e| e.to_str())
                .ok_or_else(|| (StatusCode::BAD_REQUEST, "Unknown file type".to_string()))?;

            // Get decoder
            let decoder = decoder_for_extension(extension).ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Unsupported format: {}", extension),
                )
            })?;

            // Read file data
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

            // Decode the book in a blocking task (CPU-intensive operation)
            let data_vec = data.to_vec();
            let book = tokio::task::spawn_blocking(move || {
                let mut cursor = Cursor::new(data_vec);
                decoder.decode(&mut cursor)
            })
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Task failed: {}", e),
                )
            })?
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to decode: {}", e)))?;

            let id = book.id.to_string();
            let title = book.metadata.title.clone();

            // Save book IR to storage
            let book_json = serde_json::to_string_pretty(&book)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let book_path = state
                .book_path(&id)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            tokio::fs::write(&book_path, book_json)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            // Add to library index
            {
                let mut library = state.library.write().await;
                library.books.insert(
                    id.clone(),
                    BookEntry {
                        id: id.clone(),
                        title: title.clone(),
                        authors: book.metadata.creator.clone(),
                        language: book.metadata.language.clone(),
                        description: book.metadata.description.clone(),
                        chapters: book.chapters.len(),
                        added_at: chrono::Utc::now(),
                    },
                );
            }

            // Save library index
            state
                .save_library()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            // Broadcast event
            state.broadcast(ServerEvent::BookUploaded {
                id: id.clone(),
                title: title.clone(),
            });

            return Ok(Json(UploadResponse {
                id,
                title,
                message: "Book uploaded successfully".to_string(),
            }));
        }
    }

    Err((StatusCode::BAD_REQUEST, "No file provided".to_string()))
}

/// Delete a book
pub async fn delete_book(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // Validate UUID
    Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Get paths first (validates ID format)
    let book_path = state.book_path(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Remove from library
    let entry = {
        let mut library = state.library.write().await;
        library.books.remove(&id)
    };

    let entry = match entry {
        Some(e) => e,
        None => return Err(StatusCode::NOT_FOUND),
    };

    // Save library index first (before deleting files)
    // If save fails, restore the entry for consistency
    if let Err(e) = state.save_library().await {
        // Restore the entry on failure
        let mut library = state.library.write().await;
        library.books.insert(id.clone(), entry);
        tracing::error!("Failed to save library after delete: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Now safe to delete files (library already updated)
    let _ = tokio::fs::remove_file(&book_path).await;

    // Delete cached conversions
    for format in &["epub", "typ"] {
        if let Ok(cache_path) = state.cache_path(&id, format) {
            let _ = tokio::fs::remove_file(&cache_path).await;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Download query parameters
#[derive(Debug, Deserialize)]
pub struct DownloadQuery {
    /// Output format (typst, epub)
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "epub".to_string()
}

/// Sanitize a filename for Content-Disposition header
fn sanitize_filename(name: &str, max_len: usize) -> String {
    name.chars()
        .take(max_len)
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_' || *c == '.')
        .collect::<String>()
        .trim()
        .to_string()
}

/// Download a book in the requested format
pub async fn download_book(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, StatusCode> {
    // Validate UUID
    Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Validate format
    let format = query.format.to_lowercase();
    let format = match format.as_str() {
        "pdf" => "typ", // PDF outputs Typst source
        "typst" | "typ" => "typ",
        "epub" => "epub",
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Get paths (validates ID format)
    let cache_path = state
        .cache_path(&id, format)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let book_path = state.book_path(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Check cache first (using async try_exists)
    if tokio::fs::try_exists(&cache_path).await.unwrap_or(false) {
        let data = tokio::fs::read(&cache_path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Load book metadata for consistent filename
        let book_data = tokio::fs::read_to_string(&book_path)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
        let book: Book =
            serde_json::from_str(&book_data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let content_type = match format {
            "epub" => "application/epub+zip",
            "typ" => "text/x-typst",
            _ => "application/octet-stream",
        };
        let filename = format!("{}.{}", sanitize_filename(&book.metadata.title, 50), format);

        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            )
            .body(data.into())
            .unwrap());
    }

    // Load book from storage
    let book_data = tokio::fs::read_to_string(&book_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let book: Book =
        serde_json::from_str(&book_data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get encoder
    let encoder = encoder_for_format(format).ok_or(StatusCode::BAD_REQUEST)?;

    // Encode book in a blocking task (CPU-intensive operation)
    let output = tokio::task::spawn_blocking(move || {
        let mut output = Vec::new();
        encoder.encode(&book, &mut output)?;
        Ok::<_, bookle_core::error::ConversionError>(output)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Cache the result
    let _ = tokio::fs::write(&cache_path, &output).await;

    // Broadcast event
    state.broadcast(ServerEvent::ConversionComplete {
        id: id.clone(),
        format: format.to_string(),
    });

    // Load book again for metadata (since we moved it into spawn_blocking)
    let book_data = tokio::fs::read_to_string(&book_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let book: Book =
        serde_json::from_str(&book_data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let content_type = match format {
        "epub" => "application/epub+zip",
        "typ" => "text/x-typst",
        _ => "application/octet-stream",
    };
    let ext = match format {
        "epub" => "epub",
        "typ" => "typ",
        _ => format,
    };
    let filename = format!("{}.{}", sanitize_filename(&book.metadata.title, 50), ext);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(output.into())
        .unwrap())
}
