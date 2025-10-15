use axum::{RequestExt, Router};
use matrix_sdk::stream::StreamExt;

use crate::{ client::Appservice, Config };

async fn 

pub async fn serve_appservice(service: Appservice) -> crate::Result<()> {
    let handler = Router::new()
        .fallback(handle_service)
        .with_state(service.clone())
        .into_make_service();
    println!("Hosting appservice...");
    axum_server::bind(service.config().local_address()).serve(handler).await.expect("Failed to host appservice");
    Ok(())
}
