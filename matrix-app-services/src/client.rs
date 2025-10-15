use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use parking_lot::{Mutex, RwLock};
use rcgen::CertifiedKey;
use reqwest::Certificate;
use serde::{de::DeserializeOwned, Serialize};
use tokio::{ sync::OnceCell, task::JoinHandle };

use crate::{types::{user::UserRecord, ProxyDirective, ProxyDirectiveTarget}, virtual_client::VirtualClientBuilder, Config, VirtualClient};

/// Appservice management instance
#[derive(Debug, Clone)]
pub struct Appservice {
    config: Config,
    web_server: OnceCell<Arc<Mutex<JoinHandle<crate::Result<()>>>>>,
    proxy_server: OnceCell<Arc<Mutex<JoinHandle<crate::Result<()>>>>>,
    proxy_port: u16,
    certificate: String,
    signing_key: String,
    state: sled::Db,
    proxy_token: String,
    clients: Arc<RwLock<HashMap<String, crate::VirtualClient>>>,
    proxy_directives: Arc<RwLock<HashMap<ProxyDirectiveTarget, ProxyDirective>>>
}

impl Appservice {
    /// Gets the configuration of this Appservice
    pub fn config(&self) -> Config {
        self.config.clone()
    }

    /// Gets a specific state collection
    pub(crate) fn state<V: Serialize + DeserializeOwned>(&self, collection: impl AsRef<str>) -> crate::Result<crate::types::State<V>> {
        Ok(crate::types::State::new(self.state.open_tree(collection.as_ref().as_bytes())?))
    }

    /// Gets a custom state (separated to explicitly not conflict with internal state)
    pub fn custom_state<V: Serialize + DeserializeOwned>(&self, collection: impl AsRef<str>) -> crate::Result<crate::types::State<V>> {
        self.state::<V>(format!("custom/{}", collection.as_ref()))
    }

    pub(crate) fn proxy_token(&self) -> String {
        self.proxy_token.clone()
    }

    /// Creates a new appservice from
    pub fn new(config: Config) -> crate::Result<Self> {
        rustls::crypto::ring::default_provider().install_default().unwrap();
        let CertifiedKey { cert, signing_key } = rcgen::generate_simple_self_signed(
            vec!["localhost".to_string()]
        )?;
        let cert = cert.pem();
        let signing_key = signing_key.serialize_pem();

        let proxy_port = config.proxy_ports().pick();
        let state = match config.persist_state() {
            Some(path) => sled::open(path)?,
            None => sled::Config::new().temporary(true).open()?,
        };

        let service = Appservice {
            config: config.clone(),
            web_server: OnceCell::new(),
            proxy_server: OnceCell::new(),
            proxy_port: proxy_port.clone(),
            certificate: cert.clone(),
            signing_key: signing_key.clone(),
            state,
            proxy_token: crate::generate_key(128),
            clients: Arc::new(RwLock::new(HashMap::new())),
            proxy_directives: Arc::new(RwLock::new(HashMap::new()))
        };

        Ok(service)
    }

    /// Start the associated servers, if they're not already online.
    pub fn serve(&self) -> () {
        if self.proxy_server.initialized() {
            return;
        }
        let config = self.config();
        let clonable_service = self.clone();
        println!("Attempting to serve...");

        if config.url().is_some() {
            self.web_server
                .set(
                    Arc::new(
                        Mutex::new(
                            tokio::spawn(
                                crate::servers::appservice::serve_appservice(
                                    clonable_service.clone()
                                )
                            )
                        )
                    )
                )
                .unwrap();
        }

        self.proxy_server
            .set(
                Arc::new(
                    Mutex::new(
                        tokio::spawn(
                            crate::servers::proxy::serve_proxy(
                                clonable_service.clone(),
                                clonable_service.proxy_port.clone(),
                                clonable_service.certificate.clone(),
                                clonable_service.signing_key.clone()
                            )
                        )
                    )
                )
            )
            .unwrap();
    }

    pub(crate) fn state_user_records(&self) -> crate::Result<crate::types::State<UserRecord>> {
        self.state::<UserRecord>("internal/user_records")
    }

    pub(crate) fn store_client(&self, client: VirtualClient) -> () {
        let mut clients = self.clients.write();
        let _ = clients.insert(client.localpart(), client);
    }

    pub(crate) fn retrieve_client(&self, localpart: String) -> Option<VirtualClient> {
        let clients = self.clients.read();
        clients.get(&localpart).and_then(|v| Some(v.clone()))
    }

    pub(crate) fn add_proxy_directive(&self, target: ProxyDirectiveTarget, directive: ProxyDirective) -> () {
        let mut directives = self.proxy_directives.write();
        let _ = directives.insert(target, directive);
    }

    pub(crate) fn get_proxy_directive(&self, target: ProxyDirectiveTarget) -> Option<ProxyDirective> {
        let mut directives = self.proxy_directives.write();
        directives.remove(&target)
    }
}

impl Appservice {
    pub(crate) async fn configure_service_client(
        &self,
        matrix_client: Option<matrix_sdk::ClientBuilder>,
        http_client: Option<reqwest::ClientBuilder>
    ) -> crate::Result<matrix_sdk::Client> {
        let mut headers = reqwest::header::HeaderMap::new();
        let _ = headers.insert("x-proxy-role", reqwest::header::HeaderValue::from_str("SERVICE").unwrap());
        let _ = headers.insert("x-proxy-token", reqwest::header::HeaderValue::from_str(self.proxy_token().as_str()).unwrap());
        println!("INNER_CONF");
        let client = matrix_client
            .unwrap_or(matrix_sdk::Client::builder())
            .http_client(
                http_client.unwrap_or(reqwest::Client::builder())
                .add_root_certificate(Certificate::from_pem(self.certificate.as_bytes()).unwrap())
                .default_headers(headers)
                .dns_resolver(Arc::new(crate::types::proxy::ProxyResolver::new(self.proxy_port)))
                .user_agent(self.config().user_agent())
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true)
                .build()?
            )
            .server_name(&matrix_sdk::ruma::ServerName::parse(self.config().server_name()).unwrap())
            .build().await?;

        Ok(client)
    }

    pub(crate) async fn configure_bot_client(
        &self,
        localpart: impl AsRef<str>,
        matrix_client: Option<matrix_sdk::ClientBuilder>,
        http_client: Option<reqwest::ClientBuilder>
    ) -> crate::Result<matrix_sdk::Client> {
        let localpart = localpart.as_ref().to_string();
        let state = self.state_user_records()?;
        if let Some(user) = state.get(localpart.clone())? {
            let mut headers = reqwest::header::HeaderMap::new();
            let _ = headers.insert("x-proxy-role", reqwest::header::HeaderValue::from_str("BOT").unwrap());
            let _ = headers.insert("x-proxy-token", reqwest::header::HeaderValue::from_str(self.proxy_token().as_str()).unwrap());
            let _ = headers.insert("x-proxy-bot-token", reqwest::header::HeaderValue::from_str(user.token().as_str()).unwrap());
            let _ = headers.insert("x-proxy-bot-user", reqwest::header::HeaderValue::from_str(&localpart).unwrap());

            Ok(matrix_client
                .unwrap_or(matrix_sdk::Client::builder())
                .http_client(
                    http_client.unwrap_or(reqwest::Client::builder())
                    .add_root_certificate(Certificate::from_pem(self.certificate.as_bytes()).unwrap())
                    .default_headers(headers)
                    .dns_resolver(Arc::new(crate::types::proxy::ProxyResolver::new(self.proxy_port)))
                    .user_agent(self.config().user_agent())
                    .danger_accept_invalid_certs(true)
                    .danger_accept_invalid_hostnames(true)
                    .build()?
                )
                .server_name(&matrix_sdk::ruma::ServerName::parse(self.config().server_name()).unwrap())
                .build().await?)
        } else {
            Err(crate::Error::UnregisteredUser(localpart))
        }
    }

    /// Creates a builder for a service (non-bot, using the sender_localpart) client
    pub fn build_service_client(&self) -> VirtualClientBuilder {
        VirtualClient::builder(self.clone(), self.config().sender_localpart())
    }

    /// Creates a builder for a bot client
    pub fn build_bot_client(&self, localpart: impl Into<String>) -> VirtualClientBuilder  {
        VirtualClient::builder(self.clone(), localpart)
    }
}
