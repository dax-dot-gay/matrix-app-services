use std::{fmt::Debug, io};

/// Application-specific errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An unknown error occurred
    #[error("An unexpected error occurred: {0:?}")]
    Unknown(#[from] anyhow::Error),

    /// Error parsing URL
    #[error("Error parsing \"{url}\" to URL: {err:?}")]
    UrlParsing {
        ///
        url: String,

        ///
        err: url::ParseError,
    },

    /// Error in the matrix-sdk crate
    #[error("Internal matrix SDK error: {0:?}")]
    MatrixSdk(#[from] matrix_sdk::Error),

    /// Sled persistence error
    #[error("Internal sled persistence error: {0:?}")]
    Sled(#[from] sled::Error),

    /// IOError
    #[error(transparent)]
    Io(#[from] io::Error),

    /// Reqwest error
    #[error("Encountered an issue in reqwest: {0:?}")]
    Reqwest(#[from] reqwest::Error),

    /// The requested user has not yet been registered/set up
    #[error("Unregistered user: {0}")]
    UnregisteredUser(String)
}

#[allow(missing_docs)]
impl Error {
    pub(crate) fn url_parsing(url: impl Into<String>, error: url::ParseError) -> Self {
        Self::UrlParsing { url: url.into(), err: error }
    }
}

impl From<rcgen::Error> for Error {
    fn from(value: rcgen::Error) -> Self {
        Self::Unknown(anyhow::Error::from(value))
    }
}

impl From<matrix_sdk::ClientBuildError> for Error {
    fn from(value: matrix_sdk::ClientBuildError) -> Self {
        Self::Unknown(anyhow::Error::from(value))
    }
}

impl<T: Debug + Send + Sync + 'static> From<ciborium::ser::Error<T>> for Error {
    fn from(value: ciborium::ser::Error<T>) -> Self {
        Self::Unknown(anyhow::Error::from(value))
    }
}

impl<T: Debug + Send + Sync + 'static> From<ciborium::de::Error<T>> for Error {
    fn from(value: ciborium::de::Error<T>) -> Self {
        Self::Unknown(anyhow::Error::from(value))
    }
}

///
pub type Result<T> = std::result::Result<T, Error>;
