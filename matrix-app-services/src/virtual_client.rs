use std::ops::Deref;

use matrix_sdk::{
    authentication::matrix::MatrixSession as Session,
    ruma,
    Client,
    ClientBuilder,
    SessionMeta,
    SessionTokens,
};
use serde::{ Deserialize, Serialize };

/// Whether this virtual client is a bot or the service user
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VirtualClientKind {
    /// This client is a bot user (sets user_id)
    Bot,

    /// This client is the service user (uses sender_localpart)
    #[default]
    Service,
}

/// Builder for a [`VirtualClient`]
#[derive(Debug)]
pub struct VirtualClientBuilder {
    service: crate::Appservice,
    localpart: String,
    device_id: Option<ruma::OwnedDeviceId>,
    client_builder: ClientBuilder,
    http_client_builder: Option<reqwest::ClientBuilder>,
    log_in: bool,
    create_new: bool,
    restored_session: Option<Session>,
}

impl VirtualClientBuilder {
    /// Create a new client builder
    pub fn new(service: crate::Appservice, localpart: String) -> Self {
        Self {
            service,
            localpart,
            device_id: None,
            client_builder: Client::builder(),
            http_client_builder: None,
            log_in: false,
            create_new: false,
            restored_session: None,
        }
    }

    /// Set the device ID of the appservice user
    pub fn device_id(mut self, device_id: Option<ruma::OwnedDeviceId>) -> Self {
        self.device_id = device_id;
        self
    }

    /// Sets the client builder to use for the appservice user
    pub fn client_builder(mut self, client_builder: ClientBuilder) -> Self {
        self.client_builder = client_builder;
        self
    }

    /// Sets a custom HTTP client builder
    pub fn http_client_builder(mut self, http_client_builder: reqwest::ClientBuilder) -> Self {
        self.http_client_builder = Some(http_client_builder);
        self
    }

    /// Log in as the appservice user
    pub fn login(mut self) -> Self {
        self.log_in = true;
        self
    }

    /// Force the creation of a new client (replaces any old clients for this localpart)
    pub fn create_new(mut self) -> Self {
        self.create_new = true;
        self
    }

    /// Restore a persisted session
    pub fn restored_session(mut self, session: Session) -> Self {
        self.restored_session = Some(session);
        self
    }

    /// Build the resulting VirtualClient
    pub async fn build(self) -> crate::Result<VirtualClient> {
        println!("Building...");
        if !self.create_new {
            if let Some(client) = self.service.retrieve_client(self.localpart.clone()) {
                return Ok(client);
            }
        }

        let user_id = ruma::UserId::parse_with_server_name(
            self.localpart.clone().as_str(),
            &ruma::ServerName::parse(self.service.config().server_name())?
        )?;
        let client_kind = if self.localpart == self.service.config().sender_localpart() {
            VirtualClientKind::Service
        } else {
            VirtualClientKind::Bot
        };

        println!("Configuring...");
        let internal_client = match client_kind {
            VirtualClientKind::Bot =>
                self.service.configure_bot_client(
                    self.localpart.clone(),
                    Some(self.client_builder),
                    self.http_client_builder
                ).await?,
            VirtualClientKind::Service =>
                self.service.configure_service_client(
                    Some(self.client_builder),
                    self.http_client_builder
                ).await?,
        };

        println!("Setting up session");
        let session = if let Some(session) = self.restored_session {
            session
        } else if self.log_in && client_kind != VirtualClientKind::Service {
            let login_info = ruma::api::client::session::login::v3::LoginInfo::ApplicationService(
                ruma::api::client::session::login::v3::ApplicationService::new(
                    ruma::api::client::uiaa::UserIdentifier::UserIdOrLocalpart(
                        self.localpart.clone()
                    )
                )
            );

            let request =
                ruma::assign!(ruma::api::client::session::login::v3::Request::new(login_info), {
                device_id: self.device_id,
                initial_device_display_name: None,
            });

            let response = internal_client.send(request).await?;

            Session::from(&response)
        } else {
            Session {
                meta: SessionMeta {
                    user_id: user_id.clone(),
                    device_id: self.device_id.unwrap_or_else(ruma::DeviceId::new),
                },
                tokens: SessionTokens {
                    access_token: self.service.config().appservice_token(),
                    refresh_token: None,
                },
            }
        };

        internal_client.restore_session(session).await?;

        let output = VirtualClient {
            localpart: self.localpart.clone(),
            service: self.service.clone(),
            client: internal_client,
            kind: client_kind,
        };
        self.service.store_client(output.clone());

        Ok(output)
    }
}

#[derive(Clone, Debug)]
pub struct VirtualClient {
    pub(crate) localpart: String,
    pub(crate) service: crate::Appservice,
    pub(crate) client: Client,
    pub(crate) kind: VirtualClientKind,
}

impl VirtualClient {
    /// Create a new [`VirtualClientBuilder`]
    pub fn builder(
        service: crate::Appservice,
        localpart: impl Into<String>
    ) -> VirtualClientBuilder {
        VirtualClientBuilder::new(service, localpart.into())
    }

    /// Return this client's localpart
    pub fn localpart(&self) -> String {
        self.localpart.clone()
    }

    /// Whether this client is a bot user or a service user
    pub fn kind(&self) -> VirtualClientKind {
        self.kind.clone()
    }
}

impl Deref for VirtualClient {
    type Target = Client;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}
