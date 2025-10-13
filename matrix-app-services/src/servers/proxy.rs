use std::net::SocketAddr;

use axum::{http, Router};

use crate::client::Appservice;

async fn handle_proxy(request: axum::extract::Request) -> axum::response::Response {
    println!("REQ: {request:?}");
    http::Response::new("test_reply".into())
}

pub async fn serve_proxy(
    service: Appservice,
    proxy_port: u16,
    cert: String,
    key: String
) -> crate::Result<()> {
    let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem(cert.into_bytes(), key.into_bytes()).await.expect("Failed to configure proxy TLS");
    let handler = Router::new().fallback(handle_proxy).with_state(service).into_make_service();
    axum_server::bind_rustls(SocketAddr::from(([127, 0, 0, 1], proxy_port)), tls_config).serve(handler).await?;
    Ok(())
}
