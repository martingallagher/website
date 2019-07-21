/// Generic website error.
#[derive(Debug)]
crate enum Error {
    Config(config::ConfigError),
    CssParse(String),
    HeaderValue(actix_web::http::header::InvalidHeaderValueBytes),
    Io(std::io::Error),
    None(std::option::NoneError),
    Regex(regex::Error),
    Str(&'static str),
    Utf8(std::str::Utf8Error),
}

impl std::string::ToString for Error {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl From<config::ConfigError> for Error {
    fn from(e: config::ConfigError) -> Error {
        Error::Config(e)
    }
}

impl From<actix_web::http::header::InvalidHeaderValueBytes> for Error {
    fn from(e: actix_web::http::header::InvalidHeaderValueBytes) -> Error {
        Error::HeaderValue(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<std::option::NoneError> for Error {
    fn from(e: std::option::NoneError) -> Error {
        Error::None(e)
    }
}

impl From<regex::Error> for Error {
    fn from(e: regex::Error) -> Error {
        Error::Regex(e)
    }
}

impl From<&'static str> for Error {
    fn from(e: &'static str) -> Error {
        Error::Str(e)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Error {
        Error::Utf8(e)
    }
}
