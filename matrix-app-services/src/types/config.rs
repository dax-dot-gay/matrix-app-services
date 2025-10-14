use std::{ net::SocketAddr, ops::Range, path::PathBuf };

use bon::Builder;
use getset::CloneGetters;
use ruma::api::appservice as ruma_as;
use serde::{ Deserialize, Serialize };
use url::Url;

/// An enum defining the possible types of [Namespace]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum NamespaceKind {
    #[serde(alias = "aliases")]
    Alias,

    #[serde(alias = "rooms")]
    Room,

    #[serde(alias = "users")]
    User,
}

/// A single appservice namespace
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Namespace {
    /// What kind of namespace this is
    pub kind: NamespaceKind,

    /// A POSIX regular expression defining which values this namespace includes.
    pub regex: String,

    /// A true or false value stating whether this application service has exclusive access to events within this namespace. Defaults to `false`.
    #[serde(default)]
    pub exclusive: bool,
}

impl Namespace {
    /// Creates a new [Namespace]
    pub fn new(kind: NamespaceKind, regex: impl Into<String>, exclusive: bool) -> Self {
        Self { kind, regex: regex.into(), exclusive }
    }

    /// Creates a new exclusive alias [Namespace].
    pub fn alias(regex: impl Into<String>) -> Self {
        Self { kind: NamespaceKind::Alias, regex: regex.into(), exclusive: true }
    }

    /// Creates a new exclusive room [Namespace].
    pub fn room(regex: impl Into<String>) -> Self {
        Self { kind: NamespaceKind::Room, regex: regex.into(), exclusive: true }
    }

    /// Creates a new exclusive user [Namespace].
    pub fn user(regex: impl Into<String>) -> Self {
        Self { kind: NamespaceKind::User, regex: regex.into(), exclusive: true }
    }
}

/// A range of ports (inclusive)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(missing_docs)]
pub struct PortRange {
    pub low: u16,
    pub high: u16,
}

impl PortRange {
    /// Gets an open port in the specified range
    pub fn pick(&self) -> u16 {
        openport
            ::pick_unused_port(Range::<u16>::from(self.clone()))
            .expect("No ports open in the specified range")
    }
}

impl Default for PortRange {
    fn default() -> Self {
        (50000..65000).into()
    }
}

impl From<Range<u16>> for PortRange {
    fn from(value: Range<u16>) -> Self {
        Self { low: value.start, high: value.end }
    }
}

impl<L: Into<u16>, H: Into<u16>> From<(L, H)> for PortRange {
    fn from((low, high): (L, H)) -> Self {
        Self { low: low.into(), high: high.into() }
    }
}

impl From<PortRange> for Range<u16> {
    fn from(value: PortRange) -> Self {
        value.low..value.high + 1
    }
}

/// Global configuration for the AppService
#[derive(Serialize, Deserialize, Clone, Debug, Builder, CloneGetters)]
#[getset(get_clone = "pub")]
#[builder(finish_fn(vis = "", name = build_internal))]
pub struct Config {
    /// A unique, user-defined ID of the application service which will never change.
    #[builder(start_fn, into)]
    app_id: String,

    /// The namespaces that the application service is interested in.
    #[builder(field)]
    namespaces: Vec<Namespace>,

    /// The external protocols which the application service provides (e.g. IRC).
    #[builder(field)]
    #[serde(default)]
    protocols: Vec<String>,

    /// A secret token that the application service will use to authenticate requests to the homeserver. By default, a new token is generated whenever Config is built.
    #[builder(default = Config::generate_key(32), into)]
    #[serde(alias = "as_token")]
    appservice_token: String,

    /// A secret token that the homeserver will use authenticate requests to the application service. By default, a new token is generated whenever Config is built.
    #[builder(default = Config::generate_key(32), into)]
    #[serde(alias = "hs_token")]
    homeserver_token: String,

    /// Whether requests from masqueraded users are rate-limited. The sender is excluded.
    #[builder(default)]
    #[serde(default)]
    rate_limited: bool,

    /// Whether the application service wants to receive ephemeral data.
    #[builder(default)]
    #[serde(default)]
    receive_ephemeral: bool,

    /// The localpart of the user associated with the application service. Events will be sent to the AS if this user is the target of the event, or is a joined member of the room where the event occurred.
    #[builder(into)]
    sender_localpart: String,

    /// The URL for the application service. May include a path after the domain name. Optionally set to `None` if no traffic is required.
    #[builder(into)]
    url: Option<String>,

    /// What address to bind the local server to. Ignored if `url` is `None`.
    #[builder(into, default = ([0,0,0,0], 8080))]
    local_address: SocketAddr,

    /// Ports to allow the internal proxy to bind to
    #[builder(into, default)]
    proxy_ports: PortRange,

    /// User agent string. Will default to `<application-id>/matrix-app-services:<library version>`
    #[builder(into, default)]
    user_agent: String,

    /// URL of homeserver (http(s)://...)
    #[builder(into)]
    homeserver: String,

    /// URL of an external proxy to connect through (after internal proxy handling)
    #[builder(into)]
    proxy: Option<String>,

    /// Persistent state path. Defaults to a temporary file if not provided.
    #[builder(into)]
    persist_state: Option<PathBuf>
}

impl<S: config_builder::State> ConfigBuilder<S> {
    /// Adds a single namespace to this config
    pub fn namespace(mut self, namespace: Namespace) -> Self {
        self.namespaces.push(namespace);
        self
    }

    /// Adds several namespaces to this config
    pub fn namespaces(mut self, namespaces: impl IntoIterator<Item = Namespace>) -> Self {
        self.namespaces.extend(namespaces);
        self
    }

    /// Adds a single protocol to this config
    pub fn protocol(mut self, protocol: impl Into<String>) -> Self {
        self.protocols.push(protocol.into());
        self
    }

    /// Adds several protocols to this config
    pub fn protocols(mut self, protocols: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.protocols.extend(protocols.into_iter().map(|v| v.into()));
        self
    }
}

impl<S: config_builder::IsComplete> ConfigBuilder<S> {
    /// Builds the final [Config]
    pub fn build(self) -> Config {
        let mut config = self.build_internal();
        if config.user_agent.is_empty() {
            config.user_agent = format!("{}/matrix-app-services:{}", config.app_id(), env!("CARGO_PKG_VERSION"));
        }

        config
    }
}

impl Config {
    /// Generate a secure random key
    pub fn generate_key(length: usize) -> String {
        crate::generate_key(length)
    }

    /// Generate an AppserviceRegistration
    pub fn registration(&self) -> ruma_as::Registration {
        let mut namespaces = ruma_as::Namespaces::new();
        for i in self.namespaces() {
            let ns = ruma_as::Namespace::new(i.exclusive, i.regex.clone());
            match i.kind {
                NamespaceKind::Alias => namespaces.aliases.push(ns),
                NamespaceKind::Room => namespaces.rooms.push(ns),
                NamespaceKind::User => namespaces.users.push(ns),
            }
        }

        let reginit = ruma_as::RegistrationInit {
            id: self.app_id(),
            url: self.url(),
            as_token: self.appservice_token(),
            hs_token: self.homeserver_token(),
            sender_localpart: self.sender_localpart(),
            namespaces: namespaces,
            rate_limited: Some(self.rate_limited()),
            protocols: Some(self.protocols()),
        };
        let mut registration = ruma_as::Registration::from(reginit);
        registration.receive_ephemeral = self.receive_ephemeral();
        registration
    }

    /// Get the homeserver URL
    pub fn homeserver_url(&self) -> crate::Result<Url> {
        if self.homeserver.starts_with("http") && self.homeserver.contains("://") {
            Url::parse(&self.homeserver).or_else(|e| Err(crate::Error::url_parsing(self.homeserver(), e)))
        } else {
            Url::parse(&format!("https://{}", self.homeserver())).or_else(|e| Err(crate::Error::url_parsing(self.homeserver(), e)))
        }
    }

    /// Get the homeserver name
    pub fn server_name(&self) -> String {
        let url = self.homeserver_url().expect("Expected a valid server url/name");
        url.host_str().expect("Expected a valid server name").to_string()
    }
}
