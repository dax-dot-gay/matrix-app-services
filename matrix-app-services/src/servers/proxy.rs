use std::{ net::SocketAddr, sync::Arc, usize };

use axum::{ body::Body, http, response::Response, Router };
use getset::CloneGetters;
use reqwest::header::{AUTHORIZATION, HOST};
use rustls::crypto::CryptoProvider;

use crate::client::Appservice;

type ProxyState = (reqwest::Client, Appservice);

#[derive(Clone, Debug)]
enum ProxiedEntity {
    Service {
        authorization: String
    },
    Bot {
        authorization: String,
        user_id: String
    }
}

#[derive(Clone, Debug, CloneGetters)]
#[getset(get_clone)]
struct ProxiedRequest {
    pub method: http::Method,
    pub url: url::Url,
    pub version: http::Version,
    pub headers: http::HeaderMap<http::HeaderValue>,

    #[getset(skip)]
    pub body: Arc<reqwest::Body>,
}

impl From<axum::extract::Request> for ProxiedRequest {
    fn from(value: axum::extract::Request) -> Self {
        Self {
            method: value.method().clone(),
            url: url::Url::parse(&format!("https://{}{}", value.headers().get(HOST).expect("Expected host header").to_str().unwrap(), value.uri().to_string())).expect("Unable to parse URL"),
            version: value.version(),
            headers: value.headers().clone(),
            body: Arc::new(
                reqwest::Body::wrap_stream(value.into_body().into_data_stream())
            ),
        }
    }
}

impl ProxiedRequest {
    pub fn header(&self, name: impl AsRef<str>) -> Option<String> {
        if let Some(value) = self.headers().get(name.as_ref()) {
            if let Ok(as_str) = value.to_str() {
                Some(as_str.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn into_request(self, service: Appservice, client: reqwest::Client) -> crate::Result<reqwest::Request> {
        let proxy_url = if self.url().host_str().is_some_and(|host| host.to_string() == service.config().homeserver_url().unwrap().host_str().unwrap().to_string()) {
            let mut transformed_url = self.url();
            let _ = transformed_url.set_scheme(service.config().homeserver_url().unwrap().scheme());
            transformed_url
        } else {
            self.url()
        };
        
        Ok(client.request(self.method(), proxy_url).version(self.version()).headers(self.headers()).body(Arc::into_inner(self.body).unwrap()).build()?)
    }

    pub fn verify_entity(&self, service: Appservice) -> Option<ProxiedEntity> {
        if let Some(role) = self.header("x-proxy-role") {
            match role.as_str() {
                "SERVICE" => {
                    self.header("x-proxy-token").and_then(|v| if v == service.proxy_token() {Some(ProxiedEntity::Service { authorization: service.config().appservice_token() })} else {None})
                },
                "BOT" => if self.header("x-proxy-token").is_some_and(|v| v == service.proxy_token()) {
                    if let Some(bot_token) = self.header("x-proxy-bot-token") {
                        if let Some(bot_name) = self.header("x-proxy-bot-user") {
                            if let Ok(Some(record)) = service.state_user_records().expect("Failed to get user record store").get(bot_name.clone()) {
                                if bot_token == record.token() {
                                    Some(ProxiedEntity::Bot { authorization: service.config().appservice_token(), user_id: format!("@{}:{}", bot_name, service.config().server_name()) })
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                },
                _ => None
            }
        } else {
            None
        }
    }

    pub fn authorize(mut self, entity: ProxiedEntity) -> Self {
        match entity {
            ProxiedEntity::Service { authorization } => {
                let _ = self.headers.insert(AUTHORIZATION, format!("Bearer {}", authorization).parse().unwrap());
            },
            ProxiedEntity::Bot { authorization, user_id } => {
                let _ = self.headers.insert(AUTHORIZATION, format!("Bearer {}", authorization).parse().unwrap());
                self.url.query_pairs_mut().append_pair("user_id", &user_id);
            }
        }

        self.headers = self.headers.into_iter().filter_map(|(name, value)| {
            if name.clone().is_some_and(|n| n.as_str().starts_with("x-proxy-")) {
                None
            } else if let Some(set_name) = name {
                Some((set_name, value))
            } else {
                None
            }
        }).collect();

        self
    }
}

#[axum::debug_handler]
async fn handle_proxy(
    state: axum::extract::State<ProxyState>,
    request: axum::extract::Request
) -> axum::response::Response {
    let client = state.0.0.clone();
    let service = state.1.clone();
    let request = ProxiedRequest::from(request);
    println!("PROXYING: {request:?}");
    if let Some(verified) = request.verify_entity(service.clone()) {
        let request = request.authorize(verified);
        println!("AUTHORIZED: {request:?}");
        let rqw = request.into_request(service.clone(), client.clone()).unwrap();
        match client.execute(rqw).await {
            Ok(response) => {
                let mut rsp = axum::response::Response::builder();
                if let Some(headers) = rsp.headers_mut() {
                    *headers = response.headers().clone();
                }
                rsp = rsp.status(response.status());
                rsp = rsp.version(response.version());
                let response = rsp.body(axum::body::Body::from_stream(response.bytes_stream())).unwrap();
                response

            },
            Err(e) => axum::response::Response::builder().status(500).body(format!("Internal error: {e:?}").into()).unwrap()
        }
    } else {
        axum::response::Response::builder().status(401).body("proxy.unauthorized".into()).unwrap()
    }
}

pub async fn serve_proxy(
    service: Appservice,
    proxy_port: u16,
    cert: String,
    key: String
) -> crate::Result<()> {
    let client = reqwest::Client::new();
    let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem(cert.into_bytes(), key.into_bytes()).await.expect("Failed to configure proxy TLS");
    let handler = Router::new()
        .fallback(handle_proxy)
        .with_state((client, service) as ProxyState)
        .into_make_service();
    axum_server::bind_rustls(SocketAddr::from(([127, 0, 0, 1], proxy_port)), tls_config).serve(handler).await?;
    Ok(())
}
