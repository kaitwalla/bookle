//! Server-Sent Events handler for real-time updates

use crate::state::{AppState, ServerEvent};
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::Stream;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// SSE endpoint for real-time updates
pub async fn sync_events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.subscribe();
    let stream = BroadcastStream::new(rx);

    let event_stream = stream.filter_map(|result| {
        match result {
            Ok(event) => {
                let (event_type, data) = match event {
                    ServerEvent::BookUploaded { id, title } => (
                        "book_uploaded",
                        serde_json::json!({ "id": id, "title": title }).to_string(),
                    ),
                    ServerEvent::ConversionComplete { id, format } => (
                        "conversion_complete",
                        serde_json::json!({ "id": id, "format": format }).to_string(),
                    ),
                    ServerEvent::Error { message } => (
                        "error",
                        serde_json::json!({ "message": message }).to_string(),
                    ),
                };

                Some(Ok(Event::default().event(event_type).data(data)))
            }
            Err(_) => None, // Lagged, skip
        }
    });

    Sse::new(event_stream).keep_alive(KeepAlive::default())
}
