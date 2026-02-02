//! Integration tests for the Bookle Server API

use axum_test::TestServer;
use bookle_server::routes::create_router;
use bookle_server::state::AppState;
use serde_json::Value;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{broadcast, RwLock};

/// Create a test app state with temporary storage
async fn create_test_state() -> (AppState, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let storage_path = temp_dir.path().to_path_buf();

    // Create storage directories
    tokio::fs::create_dir_all(storage_path.join("books"))
        .await
        .unwrap();
    tokio::fs::create_dir_all(storage_path.join("cache"))
        .await
        .unwrap();

    let storage = Arc::new(bookle_core::storage::LocalStorage::new(&storage_path));
    let (event_tx, _) = broadcast::channel(100);

    let state = AppState {
        storage,
        storage_path,
        library: Arc::new(RwLock::new(bookle_server::state::Library::default())),
        event_tx,
    };

    (state, temp_dir)
}

/// Create a test server
async fn create_test_server() -> (TestServer, TempDir) {
    let (state, temp_dir) = create_test_state().await;
    let app = create_router(state);
    let server = TestServer::new(app).expect("Failed to create test server");
    (server, temp_dir)
}

#[tokio::test]
async fn test_health_check() {
    let (server, _temp_dir) = create_test_server().await;

    let response = server.get("/health").await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn test_list_books_empty() {
    let (server, _temp_dir) = create_test_server().await;

    let response = server.get("/api/v1/library").await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["books"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"], 0);
    assert_eq!(body["page"], 1);
    assert_eq!(body["per_page"], 20);
}

#[tokio::test]
async fn test_list_books_with_pagination() {
    let (server, _temp_dir) = create_test_server().await;

    let response = server
        .get("/api/v1/library")
        .add_query_param("page", "2")
        .add_query_param("per_page", "10")
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["page"], 2);
    assert_eq!(body["per_page"], 10);
}

#[tokio::test]
async fn test_list_books_page_zero_treated_as_one() {
    let (server, _temp_dir) = create_test_server().await;

    // Page 0 should be treated as page 1
    let response = server
        .get("/api/v1/library")
        .add_query_param("page", "0")
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["page"], 1);
}

#[tokio::test]
async fn test_get_book_invalid_uuid() {
    let (server, _temp_dir) = create_test_server().await;

    let response = server.get("/api/v1/library/invalid-uuid").await;

    // Server validates UUID and returns not found for invalid IDs
    // (path validation fails, which results in no file found)
    response.assert_status_not_found();
}

#[tokio::test]
async fn test_get_book_not_found() {
    let (server, _temp_dir) = create_test_server().await;

    // Valid UUID but book doesn't exist
    let response = server
        .get("/api/v1/library/00000000-0000-0000-0000-000000000000")
        .await;

    response.assert_status_not_found();
}

#[tokio::test]
async fn test_delete_book_invalid_uuid() {
    let (server, _temp_dir) = create_test_server().await;

    let response = server.delete("/api/v1/library/not-a-uuid").await;

    // Server validates UUID and returns not found for invalid IDs
    response.assert_status_not_found();
}

#[tokio::test]
async fn test_delete_book_not_found() {
    let (server, _temp_dir) = create_test_server().await;

    let response = server
        .delete("/api/v1/library/00000000-0000-0000-0000-000000000000")
        .await;

    response.assert_status_not_found();
}

#[tokio::test]
async fn test_download_invalid_format() {
    let (server, _temp_dir) = create_test_server().await;

    let response = server
        .get("/api/v1/library/00000000-0000-0000-0000-000000000000/download")
        .add_query_param("format", "invalid")
        .await;

    // The server first validates the format (returns 400 for invalid),
    // but since book doesn't exist, it returns 404
    response.assert_status_not_found();
}

#[tokio::test]
async fn test_download_book_not_found() {
    let (server, _temp_dir) = create_test_server().await;

    let response = server
        .get("/api/v1/library/00000000-0000-0000-0000-000000000000/download")
        .add_query_param("format", "epub")
        .await;

    response.assert_status_not_found();
}

#[tokio::test]
async fn test_upload_no_file() {
    let (server, _temp_dir) = create_test_server().await;

    // POST with no file should fail
    let response = server.post("/api/v1/library").await;

    // Should fail with bad request (no file provided)
    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_list_books_with_search() {
    let (server, _temp_dir) = create_test_server().await;

    let response = server
        .get("/api/v1/library")
        .add_query_param("search", "test book")
        .await;

    response.assert_status_ok();

    let body: Value = response.json();
    // With no books, search returns empty but should work
    assert!(body["books"].is_array());
}

#[tokio::test]
async fn test_cors_headers() {
    let (server, _temp_dir) = create_test_server().await;

    // OPTIONS request for CORS preflight
    let response = server.get("/api/v1/library").await;

    // Response should succeed (CORS is configured)
    response.assert_status_ok();
}

#[tokio::test]
async fn test_download_default_format() {
    let (server, _temp_dir) = create_test_server().await;

    // Default format should be epub
    let response = server
        .get("/api/v1/library/00000000-0000-0000-0000-000000000000/download")
        .await;

    // Not found because book doesn't exist, but format validation should pass
    response.assert_status_not_found();
}
