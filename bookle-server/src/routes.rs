//! API routes

use crate::handlers;
use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    trace::TraceLayer,
};

/// Create the application router
pub fn create_router(state: AppState) -> Router {
    // Configure CORS based on environment
    // BOOKLE_CORS_ORIGINS can be comma-separated list of origins, or "*" for any
    let cors = match std::env::var("BOOKLE_CORS_ORIGINS").ok() {
        Some(origins) if origins == "*" => {
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        }
        Some(origins) => {
            let allowed: Vec<_> = origins
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(allowed))
                .allow_methods(Any)
                .allow_headers(Any)
        }
        None => {
            // Default: allow localhost origins for development
            CorsLayer::new()
                .allow_origin(AllowOrigin::list([
                    "http://localhost:3000".parse().unwrap(),
                    "http://localhost:5173".parse().unwrap(),
                    "http://127.0.0.1:3000".parse().unwrap(),
                    "http://127.0.0.1:5173".parse().unwrap(),
                ]))
                .allow_methods(Any)
                .allow_headers(Any)
        }
    };

    let api_routes = Router::new()
        // Library endpoints
        .route("/library", get(handlers::list_books))
        .route("/library", post(handlers::upload_book))
        .route("/library/{id}", get(handlers::get_book))
        .route("/library/{id}", axum::routing::delete(handlers::delete_book))
        .route("/library/{id}/download", get(handlers::download_book))
        // SSE endpoint
        .route("/sync", get(handlers::sync_events));

    Router::new()
        .nest("/api/v1", api_routes)
        .route("/health", get(handlers::health_check))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
