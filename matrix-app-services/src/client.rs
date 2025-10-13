use std::sync::Arc;

use parking_lot::Mutex;
use rcgen::CertifiedKey;
use sled::Tree;
use tokio::{ sync::OnceCell, task::JoinHandle };

use crate::Config;

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
}

impl Appservice {
    /// Gets the configuration of this Appservice
    pub fn config(&self) -> Config {
        self.config.clone()
    }

    /// Gets a specific state collection
    pub fn state(&self, collection: impl AsRef<str>) -> crate::Result<Tree> {
        Ok(self.state.open_tree(collection.as_ref().as_bytes())?)
    }

    /// Creates a new appservice from
    pub fn new(config: Config) -> crate::Result<Self> {
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

        let service = self;

        if config.url().is_some() {
            service.web_server
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

        service.proxy_server
            .set(
                Arc::new(
                    Mutex::new(
                        tokio::spawn(
                            crate::servers::proxy::serve_proxy(
                                clonable_service.clone(),
                                clonable_service.proxy_port.clone(),
                                clonable_service.certificate.clone(),
                                clonable_service.certificate.clone()
                            )
                        )
                    )
                )
            )
            .unwrap();
    }
}
