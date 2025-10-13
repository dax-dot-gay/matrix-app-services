use getset::CloneGetters;
use matrix_sdk::ruma::{ DeviceId, UserId, ServerName, OwnedUserId, OwnedDeviceId };
use serde::{ Deserialize, Serialize };

/// An appservice-managed user
#[derive(Serialize, Deserialize, Clone, Debug, CloneGetters)]
pub struct UserRecord {
    /// Proxy access token
    #[getset(get_clone = "pub(crate)")]
    token: String,

    /// Matrix user id
    #[getset(get_clone = "pub with_prefix")]
    user_id: String,

    /// Matrix device id
    #[getset(get_clone = "pub with_prefix")]
    device_id: String,
}

impl UserRecord {
    /// Creates a new user with a random ID localpart
    pub fn new(server_name: impl AsRef<str>) -> Self {
        Self {
            token: crate::generate_key(128),
            user_id: UserId::new(
                &ServerName::parse(server_name).expect("Expected valid server_name")
            ).to_string(),
            device_id: DeviceId::new().to_string(),
        }
    }

    /// Creates a new user with a pre-set localpart
    pub fn new_with_id(localpart: impl AsRef<str>, server_name: impl AsRef<str>) -> Self {
        Self {
            token: crate::generate_key(128),
            user_id: UserId::parse_with_server_name(
                localpart.as_ref(),
                &ServerName::parse(server_name).expect("Expected valid server_name")
            )
                .expect("Failed to parse full user id")
                .to_string(),
            device_id: DeviceId::new().to_string(),
        }
    }

    /// Gets the user_id as an [`OwnedUserId`]
    pub fn user_id(&self) -> OwnedUserId {
        UserId::parse(self.get_user_id()).expect("Should contain a valid UserId")
    }

    /// Gets the device_id as an [`OwnedDeviceId`]
    pub fn device_id(&self) -> OwnedDeviceId {
        OwnedDeviceId::from(self.get_device_id())
    }
}
