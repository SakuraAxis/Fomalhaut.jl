use futures_util::SinkExt;
use tokio::net::TcpStream;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::runtime::state::{WsSender, state};

pub fn route_exists(path: &str) -> bool {
    match state().read() {
        Ok(guard) => guard.ws_routes.contains_key(path) || guard.axis_ws_routes.contains_key(path),
        Err(_) => false,
    }
}

fn resolve_sender(path: &str) -> Option<WsSender> {
    let guard = state().read().ok()?;
    guard
        .ws_routes
        .get(path)
        .or_else(|| guard.axis_ws_routes.get(path))
        .cloned()
}

pub async fn handle_socket(path: String, stream: TcpStream) {
    let Ok(mut socket) = accept_async(stream).await else {
        return;
    };

    let tx = match resolve_sender(&path) {
        Some(tx) => tx,
        None => return,
    };

    let mut rx = tx.subscribe();

    while rx.changed().await.is_ok() {
        let frame = {
            let b = rx.borrow();
            if b.is_empty() {
                continue;
            }
            b.clone()
        };

        if socket
            .send(Message::Binary((*frame).clone().into()))
            .await
            .is_err()
        {
            break;
        }
    }
}
