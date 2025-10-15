use std::net::SocketAddr;

use reqwest::dns::{Addrs, Resolve};
use serde::{ Deserialize, Serialize };

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ProxyDirectiveTarget {
    Service {
        path: String,
    },
    Bot {
        token: String,
        path: String,
    },
}

impl ProxyDirectiveTarget {
    pub fn service(path: impl Into<String>) -> Self {
        Self::Service { path: path.into() }
    }

    pub fn bot(path: impl Into<String>, token: impl Into<String>) -> Self {
        Self::Bot { token: token.into(), path: path.into() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum ProxyDirective {
    DoNotModify,
}

#[derive(Debug)]
pub(crate) struct ProxyResolver(u16);

impl ProxyResolver {
    pub fn new(port: u16) -> Self {
        Self(port)
    }
}

impl Resolve for ProxyResolver {
    fn resolve(&self, _: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let port = self.0.clone();
        Box::pin(async move {Ok(Box::new(vec![SocketAddr::from(([127,0,0,1], port))].into_iter()) as Addrs)})
    }
}
