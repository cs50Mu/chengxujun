use axum::{routing::get, Extension, Router, Server};
use ws_server::{ws_handler, ChatState};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(Extension(ChatState::new()));
    let addr = "127.0.0.1:5000".parse().unwrap();
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
