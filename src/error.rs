use std::fmt;

/// Generic website error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    Config(#[from] config::ConfigError),
    CssParse(String),
    HeaderValue(#[from] actix_web::http::header::InvalidHeaderValueBytes),
    Io(#[from] std::io::Error),
    None,
    Regex(#[from] regex::Error),
    Utf8(#[from] std::str::Utf8Error),
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<std::option::NoneError> for Error {
    fn from(_e: std::option::NoneError) -> Self {
        Self::None
    }
}
