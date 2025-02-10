use reqwest;
use thiserror::Error;
use url::ParseError;

#[derive(Debug, Error)]
pub enum ZoteroError {
    #[error("HTTP request error: {0}")]
    HttpRequestError(#[from] reqwest::Error),
    #[error("Unsupported content type: {0}")]
    UnsupportedContentType(String),
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] ParseError),
    #[error("Header value error: {0}")]
    HeaderValueError(#[from] reqwest::header::InvalidHeaderValue),
    #[error("Too many requests: {0}")]
    TooManyRequests(String),
    #[error("Failed to retrieve file: {0}")]
    FileRetrievalError(String),
}
