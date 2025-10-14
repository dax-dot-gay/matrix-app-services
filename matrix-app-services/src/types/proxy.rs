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
