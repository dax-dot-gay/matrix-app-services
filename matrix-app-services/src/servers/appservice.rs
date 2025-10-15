use axum::{RequestExt, Router};
use matrix_sdk::stream::StreamExt;

use crate::{ client::Appservice, Config };

async fn handle_service(state: axum::extract::State<Appservice>, request: axum::extract::Request) -> axum::response::Response {
    println!("GOT AS REQUEST: {request:?}");
    let body_data: axum::body::Bytes = request.extract().await.unwrap();
    println!("BODY: {}", String::from_utf8_lossy(&body_data.to_vec()).to_string());
    axum::response::Response::new("{}".into())
}

pub async fn serve_appservice(service: Appservice) -> crate::Result<()> {
    let handler = Router::new()
        .fallback(handle_service)
        .with_state(service.clone())
        .into_make_service();
    println!("Hosting appservice...");
    axum_server::bind(service.config().local_address()).serve(handler).await.expect("Failed to host appservice");
    Ok(())
}
