//! OPDS catalog handlers
//!
//! HTTP handlers for OPDS 1.2 (Atom/XML) and OPDS 2.0 (JSON) endpoints.

use crate::opds::{
    mime, opensearch, rel, xml, Opds2Contributor, Opds2Feed, Opds2FeedMetadata, Opds2Image,
    Opds2Link, Opds2Navigation, Opds2Publication, Opds2PublicationMetadata, OpdsEntry, OpdsFeed,
    OpdsLink,
};
use crate::state::{AppState, BookEntry};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use bookle_core::Book;
use serde::Deserialize;
use uuid::Uuid;

/// Default items per page
const DEFAULT_PER_PAGE: u32 = 25;
/// Maximum items per page
const MAX_PER_PAGE: u32 = 100;

/// Pagination query parameters
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    DEFAULT_PER_PAGE
}

/// Search query parameters
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

/// Extract base URL from headers or environment
fn get_base_url(headers: &HeaderMap) -> String {
    // Check environment variable first
    if let Ok(base_url) = std::env::var("BOOKLE_BASE_URL") {
        return base_url.trim_end_matches('/').to_string();
    }

    // Try to extract from Host header
    if let Some(host) = headers.get(header::HOST).and_then(|h| h.to_str().ok()) {
        // Determine scheme (assume http for localhost, https otherwise)
        let scheme = if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
            "http"
        } else {
            "https"
        };
        return format!("{}://{}", scheme, host);
    }

    // Default for development
    "http://localhost:3000".to_string()
}

/// Sanitize pagination parameters
fn sanitize_pagination(page: u32, per_page: u32) -> (u32, u32) {
    let page = page.max(1);
    let per_page = per_page.clamp(1, MAX_PER_PAGE);
    (page, per_page)
}

// ============================================================================
// OPDS 1.2 (Atom/XML) Handlers
// ============================================================================

/// OPDS 1.2 Root Catalog (Navigation Feed)
pub async fn root_catalog(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let base_url = get_base_url(&headers);
    let library = state.library.read().await;
    let total_books = library.books.len();

    let mut feed = OpdsFeed::new(format!("urn:uuid:{}", Uuid::new_v4()), "Bookle Library")
        .with_author("Bookle");

    // Self link
    feed.add_link(OpdsLink::new(
        rel::SELF,
        format!("{}/opds", base_url),
        mime::OPDS_CATALOG_KIND_NAVIGATION,
    ));

    // Start link (same as self for root)
    feed.add_link(OpdsLink::new(
        rel::START,
        format!("{}/opds", base_url),
        mime::OPDS_CATALOG_KIND_NAVIGATION,
    ));

    // Search link
    feed.add_link(OpdsLink::new(
        rel::SEARCH,
        format!("{}/opds/opensearch.xml", base_url),
        mime::OPENSEARCH,
    ));

    // Navigation entries
    // All Books
    let mut all_entry = OpdsEntry::new(format!("urn:uuid:{}", Uuid::new_v4()), "All Books");
    all_entry.content = Some(format!("Browse all {} books in the library", total_books));
    all_entry.add_link(OpdsLink::new(
        rel::SUBSECTION,
        format!("{}/opds/all", base_url),
        mime::OPDS_CATALOG_KIND_ACQUISITION,
    ));
    feed.add_entry(all_entry);

    // Recent Books
    let mut recent_entry =
        OpdsEntry::new(format!("urn:uuid:{}", Uuid::new_v4()), "Recent Additions");
    recent_entry.content = Some("Recently added books".to_string());
    recent_entry.add_link(OpdsLink::new(
        rel::SUBSECTION,
        format!("{}/opds/recent", base_url),
        mime::OPDS_CATALOG_KIND_ACQUISITION,
    ));
    feed.add_entry(recent_entry);

    match xml::render_feed(&feed) {
        Ok(xml) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime::OPDS_CATALOG_KIND_NAVIGATION)
            .body(xml)
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to render OPDS feed: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(String::new())
                .unwrap()
        }
    }
}

/// OPDS 1.2 All Books (Acquisition Feed)
pub async fn all_books(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PaginationQuery>,
) -> impl IntoResponse {
    let base_url = get_base_url(&headers);
    let (page, per_page) = sanitize_pagination(query.page, query.per_page);

    let library = state.library.read().await;
    let mut books: Vec<&BookEntry> = library.books.values().collect();
    books.sort_by(|a, b| a.title.cmp(&b.title));

    let total = books.len() as u32;
    let total_pages = total.div_ceil(per_page);
    let start = ((page - 1) * per_page) as usize;
    let page_books: Vec<&BookEntry> = books
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .collect();

    let mut feed =
        OpdsFeed::new(format!("urn:uuid:{}", Uuid::new_v4()), "All Books").with_author("Bookle");

    // Self link
    feed.add_link(OpdsLink::new(
        rel::SELF,
        format!("{}/opds/all?page={}&per_page={}", base_url, page, per_page),
        mime::OPDS_CATALOG_KIND_ACQUISITION,
    ));

    // Start link
    feed.add_link(OpdsLink::new(
        rel::START,
        format!("{}/opds", base_url),
        mime::OPDS_CATALOG_KIND_NAVIGATION,
    ));

    // Search link
    feed.add_link(OpdsLink::new(
        rel::SEARCH,
        format!("{}/opds/opensearch.xml", base_url),
        mime::OPENSEARCH,
    ));

    // Pagination links
    if page > 1 {
        feed.add_link(OpdsLink::new(
            rel::PREVIOUS,
            format!(
                "{}/opds/all?page={}&per_page={}",
                base_url,
                page - 1,
                per_page
            ),
            mime::OPDS_CATALOG_KIND_ACQUISITION,
        ));
    }
    if page < total_pages {
        feed.add_link(OpdsLink::new(
            rel::NEXT,
            format!(
                "{}/opds/all?page={}&per_page={}",
                base_url,
                page + 1,
                per_page
            ),
            mime::OPDS_CATALOG_KIND_ACQUISITION,
        ));
    }

    // Book entries
    for book in page_books {
        feed.add_entry(book_entry_to_opds(book, &base_url));
    }

    match xml::render_feed(&feed) {
        Ok(xml) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime::OPDS_CATALOG_KIND_ACQUISITION)
            .body(xml)
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to render OPDS feed: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(String::new())
                .unwrap()
        }
    }
}

/// OPDS 1.2 Recent Books
pub async fn recent_books(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PaginationQuery>,
) -> impl IntoResponse {
    let base_url = get_base_url(&headers);
    let (page, per_page) = sanitize_pagination(query.page, query.per_page);

    let library = state.library.read().await;
    let mut books: Vec<&BookEntry> = library.books.values().collect();
    // Sort by added_at descending (most recent first)
    books.sort_by(|a, b| b.added_at.cmp(&a.added_at));

    let total = books.len() as u32;
    let total_pages = total.div_ceil(per_page);
    let start = ((page - 1) * per_page) as usize;
    let page_books: Vec<&BookEntry> = books
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .collect();

    let mut feed = OpdsFeed::new(format!("urn:uuid:{}", Uuid::new_v4()), "Recent Additions")
        .with_author("Bookle");

    // Self link
    feed.add_link(OpdsLink::new(
        rel::SELF,
        format!(
            "{}/opds/recent?page={}&per_page={}",
            base_url, page, per_page
        ),
        mime::OPDS_CATALOG_KIND_ACQUISITION,
    ));

    // Start link
    feed.add_link(OpdsLink::new(
        rel::START,
        format!("{}/opds", base_url),
        mime::OPDS_CATALOG_KIND_NAVIGATION,
    ));

    // Search link
    feed.add_link(OpdsLink::new(
        rel::SEARCH,
        format!("{}/opds/opensearch.xml", base_url),
        mime::OPENSEARCH,
    ));

    // Pagination links
    if page > 1 {
        feed.add_link(OpdsLink::new(
            rel::PREVIOUS,
            format!(
                "{}/opds/recent?page={}&per_page={}",
                base_url,
                page - 1,
                per_page
            ),
            mime::OPDS_CATALOG_KIND_ACQUISITION,
        ));
    }
    if page < total_pages {
        feed.add_link(OpdsLink::new(
            rel::NEXT,
            format!(
                "{}/opds/recent?page={}&per_page={}",
                base_url,
                page + 1,
                per_page
            ),
            mime::OPDS_CATALOG_KIND_ACQUISITION,
        ));
    }

    // Book entries
    for book in page_books {
        feed.add_entry(book_entry_to_opds(book, &base_url));
    }

    match xml::render_feed(&feed) {
        Ok(xml) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime::OPDS_CATALOG_KIND_ACQUISITION)
            .body(xml)
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to render OPDS feed: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(String::new())
                .unwrap()
        }
    }
}

/// OPDS 1.2 Search
pub async fn search_books(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let base_url = get_base_url(&headers);
    let (page, per_page) = sanitize_pagination(query.page, query.per_page);
    let search_term = query.q.unwrap_or_default();

    let library = state.library.read().await;
    let mut books: Vec<&BookEntry> = library
        .books
        .values()
        .filter(|book| {
            if search_term.is_empty() {
                return true;
            }
            let search_lower = search_term.to_lowercase();
            book.title.to_lowercase().contains(&search_lower)
                || book
                    .authors
                    .iter()
                    .any(|a| a.to_lowercase().contains(&search_lower))
        })
        .collect();
    books.sort_by(|a, b| a.title.cmp(&b.title));

    let total = books.len() as u32;
    let total_pages = total.div_ceil(per_page).max(1);
    let start = ((page - 1) * per_page) as usize;
    let page_books: Vec<&BookEntry> = books
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .collect();

    let feed_title = if search_term.is_empty() {
        "Search Results".to_string()
    } else {
        format!("Search: {}", search_term)
    };

    let mut feed =
        OpdsFeed::new(format!("urn:uuid:{}", Uuid::new_v4()), feed_title).with_author("Bookle");

    // Self link
    let self_url = if search_term.is_empty() {
        format!(
            "{}/opds/search?page={}&per_page={}",
            base_url, page, per_page
        )
    } else {
        format!(
            "{}/opds/search?q={}&page={}&per_page={}",
            base_url,
            urlencoding::encode(&search_term),
            page,
            per_page
        )
    };
    feed.add_link(OpdsLink::new(
        rel::SELF,
        self_url,
        mime::OPDS_CATALOG_KIND_ACQUISITION,
    ));

    // Start link
    feed.add_link(OpdsLink::new(
        rel::START,
        format!("{}/opds", base_url),
        mime::OPDS_CATALOG_KIND_NAVIGATION,
    ));

    // Pagination links
    if page > 1 {
        let prev_url = if search_term.is_empty() {
            format!(
                "{}/opds/search?page={}&per_page={}",
                base_url,
                page - 1,
                per_page
            )
        } else {
            format!(
                "{}/opds/search?q={}&page={}&per_page={}",
                base_url,
                urlencoding::encode(&search_term),
                page - 1,
                per_page
            )
        };
        feed.add_link(OpdsLink::new(
            rel::PREVIOUS,
            prev_url,
            mime::OPDS_CATALOG_KIND_ACQUISITION,
        ));
    }
    if page < total_pages {
        let next_url = if search_term.is_empty() {
            format!(
                "{}/opds/search?page={}&per_page={}",
                base_url,
                page + 1,
                per_page
            )
        } else {
            format!(
                "{}/opds/search?q={}&page={}&per_page={}",
                base_url,
                urlencoding::encode(&search_term),
                page + 1,
                per_page
            )
        };
        feed.add_link(OpdsLink::new(
            rel::NEXT,
            next_url,
            mime::OPDS_CATALOG_KIND_ACQUISITION,
        ));
    }

    // Book entries
    for book in page_books {
        feed.add_entry(book_entry_to_opds(book, &base_url));
    }

    match xml::render_feed(&feed) {
        Ok(xml) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime::OPDS_CATALOG_KIND_ACQUISITION)
            .body(xml)
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to render OPDS feed: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(String::new())
                .unwrap()
        }
    }
}

/// OpenSearch descriptor
pub async fn opensearch_descriptor(headers: HeaderMap) -> impl IntoResponse {
    let base_url = get_base_url(&headers);
    let xml = opensearch::render_opensearch(&base_url);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime::OPENSEARCH)
        .body(xml)
        .unwrap()
}

/// Convert BookEntry to OPDS 1.2 Entry
fn book_entry_to_opds(book: &BookEntry, base_url: &str) -> OpdsEntry {
    let mut entry = OpdsEntry::new(format!("urn:uuid:{}", book.id), &book.title);

    entry.updated = book.added_at;
    entry.language = Some(book.language.clone());

    // Authors
    for author in &book.authors {
        entry.add_author(author);
    }

    // Summary/description
    if let Some(ref desc) = book.description {
        entry.summary = Some(desc.clone());
    }

    // Acquisition link (EPUB download)
    entry.add_link(OpdsLink::new(
        rel::ACQUISITION_OPEN_ACCESS,
        format!(
            "{}/api/v1/library/{}/download?format=epub",
            base_url, book.id
        ),
        mime::EPUB,
    ));

    // Cover image link (use generic image/* since actual type is determined at runtime)
    entry.add_link(OpdsLink::new(
        rel::IMAGE,
        format!("{}/opds/cover/{}", base_url, book.id),
        mime::IMAGE,
    ));

    // Thumbnail link
    entry.add_link(OpdsLink::new(
        rel::THUMBNAIL,
        format!("{}/opds/cover/{}/thumbnail", base_url, book.id),
        mime::IMAGE,
    ));

    entry
}

// ============================================================================
// Cover Image Handlers
// ============================================================================

/// Cover image response type
pub struct CoverResponse {
    data: Vec<u8>,
    content_type: String,
}

impl IntoResponse for CoverResponse {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, self.content_type)
            .header(header::CACHE_CONTROL, "public, max-age=86400")
            .body(axum::body::Body::from(self.data))
            .unwrap()
    }
}

/// Serve cover image for a book
pub async fn cover_image(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<CoverResponse, StatusCode> {
    serve_cover(state, &id, false).await
}

/// Serve cover thumbnail for a book
pub async fn cover_thumbnail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<CoverResponse, StatusCode> {
    serve_cover(state, &id, true).await
}

/// Internal function to serve cover images
async fn serve_cover(
    state: AppState,
    id: &str,
    _thumbnail: bool, // For now, we serve the same image; could add resizing later
) -> Result<CoverResponse, StatusCode> {
    // Validate UUID
    Uuid::parse_str(id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Load book IR
    let book_path = state.book_path(id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let book_data = tokio::fs::read_to_string(&book_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let book: Book =
        serde_json::from_str(&book_data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get cover resource key
    let cover_key = book
        .metadata
        .cover_resource_key
        .as_ref()
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get cover resource
    let resource = book.resources.get(cover_key).ok_or(StatusCode::NOT_FOUND)?;

    // Get image data
    let data = resource
        .data
        .as_bytes()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(CoverResponse {
        data,
        content_type: resource.mime_type.clone(),
    })
}

// ============================================================================
// OPDS 2.0 (JSON) Handlers
// ============================================================================

/// OPDS 2.0 Root Catalog
pub async fn v2_root(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let base_url = get_base_url(&headers);
    let library = state.library.read().await;
    let total_books = library.books.len() as u32;

    let feed = Opds2Feed {
        metadata: Opds2FeedMetadata {
            title: "Bookle Library".to_string(),
            subtitle: Some("Your personal ebook library".to_string()),
            modified: Some(chrono::Utc::now()),
            number_of_items: Some(total_books),
            items_per_page: None,
            current_page: None,
        },
        links: vec![
            Opds2Link::new(rel::SELF, format!("{}/opds/v2", base_url), mime::OPDS_JSON),
            Opds2Link::new(
                rel::SEARCH,
                format!("{}/opds/opensearch.xml", base_url),
                mime::OPENSEARCH,
            ),
        ],
        publications: Vec::new(),
        navigation: vec![Opds2Navigation {
            title: "All Books".to_string(),
            href: format!("{}/opds/v2/publications", base_url),
            media_type: mime::OPDS_JSON.to_string(),
            rel: rel::SUBSECTION.to_string(),
        }],
        groups: Vec::new(),
    };

    Json(feed)
}

/// OPDS 2.0 Publications (All Books)
pub async fn v2_publications(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PaginationQuery>,
) -> impl IntoResponse {
    let base_url = get_base_url(&headers);
    let (page, per_page) = sanitize_pagination(query.page, query.per_page);

    let library = state.library.read().await;
    let mut books: Vec<&BookEntry> = library.books.values().collect();
    books.sort_by(|a, b| a.title.cmp(&b.title));

    let total = books.len() as u32;
    let total_pages = total.div_ceil(per_page).max(1);
    let start = ((page - 1) * per_page) as usize;
    let page_books: Vec<&BookEntry> = books
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .collect();

    let mut links = vec![
        Opds2Link::new(
            rel::SELF,
            format!(
                "{}/opds/v2/publications?page={}&per_page={}",
                base_url, page, per_page
            ),
            mime::OPDS_JSON,
        ),
        Opds2Link::new(rel::START, format!("{}/opds/v2", base_url), mime::OPDS_JSON),
    ];

    // Pagination links
    if page > 1 {
        links.push(Opds2Link::new(
            rel::PREVIOUS,
            format!(
                "{}/opds/v2/publications?page={}&per_page={}",
                base_url,
                page - 1,
                per_page
            ),
            mime::OPDS_JSON,
        ));
    }
    if page < total_pages {
        links.push(Opds2Link::new(
            rel::NEXT,
            format!(
                "{}/opds/v2/publications?page={}&per_page={}",
                base_url,
                page + 1,
                per_page
            ),
            mime::OPDS_JSON,
        ));
    }

    let publications: Vec<Opds2Publication> = page_books
        .iter()
        .map(|book| book_entry_to_opds2(book, &base_url))
        .collect();

    let feed = Opds2Feed {
        metadata: Opds2FeedMetadata {
            title: "All Books".to_string(),
            subtitle: None,
            modified: Some(chrono::Utc::now()),
            number_of_items: Some(total),
            items_per_page: Some(per_page),
            current_page: Some(page),
        },
        links,
        publications,
        navigation: Vec::new(),
        groups: Vec::new(),
    };

    Json(feed)
}

/// OPDS 2.0 Search
pub async fn v2_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let base_url = get_base_url(&headers);
    let (page, per_page) = sanitize_pagination(query.page, query.per_page);
    let search_term = query.q.unwrap_or_default();

    let library = state.library.read().await;
    let mut books: Vec<&BookEntry> = library
        .books
        .values()
        .filter(|book| {
            if search_term.is_empty() {
                return true;
            }
            let search_lower = search_term.to_lowercase();
            book.title.to_lowercase().contains(&search_lower)
                || book
                    .authors
                    .iter()
                    .any(|a| a.to_lowercase().contains(&search_lower))
        })
        .collect();
    books.sort_by(|a, b| a.title.cmp(&b.title));

    let total = books.len() as u32;
    let total_pages = total.div_ceil(per_page).max(1);
    let start = ((page - 1) * per_page) as usize;
    let page_books: Vec<&BookEntry> = books
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .collect();

    let self_url = if search_term.is_empty() {
        format!(
            "{}/opds/v2/search?page={}&per_page={}",
            base_url, page, per_page
        )
    } else {
        format!(
            "{}/opds/v2/search?q={}&page={}&per_page={}",
            base_url,
            urlencoding::encode(&search_term),
            page,
            per_page
        )
    };

    let mut links = vec![
        Opds2Link::new(rel::SELF, self_url, mime::OPDS_JSON),
        Opds2Link::new(rel::START, format!("{}/opds/v2", base_url), mime::OPDS_JSON),
    ];

    // Pagination links
    if page > 1 {
        let prev_url = if search_term.is_empty() {
            format!(
                "{}/opds/v2/search?page={}&per_page={}",
                base_url,
                page - 1,
                per_page
            )
        } else {
            format!(
                "{}/opds/v2/search?q={}&page={}&per_page={}",
                base_url,
                urlencoding::encode(&search_term),
                page - 1,
                per_page
            )
        };
        links.push(Opds2Link::new(rel::PREVIOUS, prev_url, mime::OPDS_JSON));
    }
    if page < total_pages {
        let next_url = if search_term.is_empty() {
            format!(
                "{}/opds/v2/search?page={}&per_page={}",
                base_url,
                page + 1,
                per_page
            )
        } else {
            format!(
                "{}/opds/v2/search?q={}&page={}&per_page={}",
                base_url,
                urlencoding::encode(&search_term),
                page + 1,
                per_page
            )
        };
        links.push(Opds2Link::new(rel::NEXT, next_url, mime::OPDS_JSON));
    }

    let publications: Vec<Opds2Publication> = page_books
        .iter()
        .map(|book| book_entry_to_opds2(book, &base_url))
        .collect();

    let feed_title = if search_term.is_empty() {
        "Search Results".to_string()
    } else {
        format!("Search: {}", search_term)
    };

    let feed = Opds2Feed {
        metadata: Opds2FeedMetadata {
            title: feed_title,
            subtitle: None,
            modified: Some(chrono::Utc::now()),
            number_of_items: Some(total),
            items_per_page: Some(per_page),
            current_page: Some(page),
        },
        links,
        publications,
        navigation: Vec::new(),
        groups: Vec::new(),
    };

    Json(feed)
}

/// Convert BookEntry to OPDS 2.0 Publication
fn book_entry_to_opds2(book: &BookEntry, base_url: &str) -> Opds2Publication {
    Opds2Publication {
        metadata: Opds2PublicationMetadata {
            title: book.title.clone(),
            author: book
                .authors
                .iter()
                .map(|name| Opds2Contributor { name: name.clone() })
                .collect(),
            language: Some(book.language.clone()),
            description: book.description.clone(),
            publisher: None,
            published: None,
            modified: Some(book.added_at),
            subject: Vec::new(),
            identifier: Some(book.id.clone()),
        },
        links: vec![Opds2Link::new(
            rel::ACQUISITION_OPEN_ACCESS,
            format!(
                "{}/api/v1/library/{}/download?format=epub",
                base_url, book.id
            ),
            mime::EPUB,
        )],
        images: vec![
            Opds2Image {
                href: format!("{}/opds/cover/{}", base_url, book.id),
                media_type: mime::IMAGE.to_string(),
                width: None,
                height: None,
            },
            Opds2Image {
                href: format!("{}/opds/cover/{}/thumbnail", base_url, book.id),
                media_type: mime::IMAGE.to_string(),
                width: Some(200),
                height: None,
            },
        ],
    }
}
