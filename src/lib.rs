#[allow(dead_code)]
#[allow(unused)]
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::Client;
use reqwest::Url;
use serde_json::Value;
use thiserror::Error;
use url::ParseError;

const VERSION: &str = "1";
const API_VERSION: &str = "3";

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
}

#[derive(Debug)]
pub struct Zotero {
    client: Client,
    api_key: String,
    endpoint: String,
    library_id: String,
    library_type: String,
    locale: Option<String>,
    max_retries: u8,
}

impl Zotero {
    pub fn user_lib(user_id: &str, api_key: &str) -> Result<Self, ZoteroError> {
        Self::new(
            user_id.to_string(),
            "users".to_string(),
            api_key.to_string(),
        )
    }

    pub fn group_lib(library_id: &str, api_key: &str) -> Result<Self, ZoteroError> {
        Self::new(
            library_id.to_string(),
            "groups".to_string(),
            api_key.to_string(),
        )
    }

    fn new(library_id: String, library_type: String, api_key: String) -> Result<Self, ZoteroError> {
        let endpoint = "https://api.zotero.org".to_string();
        Ok(Zotero {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
            api_key,
            endpoint,
            library_id,
            library_type,
            locale: Some("en-US".to_string()),
            max_retries: 5,
        })
    }

    pub fn set_endpoint(&mut self, endpoint: &str) {
        self.endpoint = endpoint.to_string();
    }

    pub fn set_locale(&mut self, locale: &str) {
        self.locale = Some(locale.to_string());
    }

    fn default_headers(&self) -> Result<HeaderMap, ZoteroError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&format!("zotero-rust/{}", VERSION))?,
        );
        headers.insert("Zotero-API-Version", HeaderValue::from_str(API_VERSION)?);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))?,
        );
        Ok(headers)
    }

    fn build_url(&self, path: &str) -> Result<Url, ZoteroError> {
        let mut url = Url::parse(&format!(
            "{}/{}/{}/{}",
            self.endpoint, self.library_type, self.library_id, path
        ))?;
        if let Some(ref loc) = self.locale {
            url.query_pairs_mut().append_pair("locale", loc);
        }
        Ok(url)
    }

    async fn handle_response(&self, url: Url) -> Result<Value, ZoteroError> {
        let mut attempts = 0;
        let mut backoff = 0.0;
        while attempts < self.max_retries {
            let response = self
                .client
                .get(url.clone())
                .headers(self.default_headers()?)
                .send()
                .await?;

            if let Some(bo) = response.headers().get("backoff") {
                if let Ok(val) = bo.to_str() {
                    if let Ok(parsed_backoff) = val.parse::<f64>() {
                        backoff = parsed_backoff;
                    }
                }
            } else if let Some(retry_after) = response.headers().get("retry-after") {
                if let Ok(val) = retry_after.to_str() {
                    if let Ok(parsed_backoff) = val.parse::<f64>() {
                        backoff = parsed_backoff;
                    }
                }
            }

            let status = response.status();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let delay_secs = backoff;
                tokio::time::sleep(std::time::Duration::from_secs_f64(delay_secs)).await;
                attempts += 1;
                continue;
            }

            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if content_type.starts_with("application/json") {
                let json: Value = response.json().await?;
                return Ok(json);
            } else if content_type.starts_with("text/html") {
                let text = response.text().await?;
                return Ok(Value::String(text));
            } else {
                return Err(ZoteroError::UnsupportedContentType(
                    content_type.to_string(),
                ));
            }
        }

        Err(ZoteroError::TooManyRequests(
            "429: Too Many Requests".to_string(),
        ))
    }

    pub async fn key_info(&self) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("keys/{}", self.api_key))?;
        self.handle_response(url).await
    }

    pub async fn top(&self) -> Result<Value, ZoteroError> {
        let url = self.build_url("items/top")?;
        self.handle_response(url).await
    }

    pub async fn collections(&self) -> Result<Value, ZoteroError> {
        let url = self.build_url("collections")?;
        self.handle_response(url).await
    }

    pub async fn collection(&self, collection_id: &str) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("collections/{}", collection_id))?;
        self.handle_response(url).await
    }

    pub async fn collections_top(&self) -> Result<Value, ZoteroError> {
        let url = self.build_url("collections/top")?;
        self.handle_response(url).await
    }

    pub async fn collections_sub(&self, collection_id: &str) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("collections/{}/collections", collection_id))?;
        self.handle_response(url).await
    }

    pub async fn collection_items(&self, collection_id: &str) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("collections/{}/items", collection_id))?;
        self.handle_response(url).await
    }

    pub async fn item(&self, item_id: &str) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("items/{}", item_id))?;
        self.handle_response(url).await
    }

    pub async fn items(&self) -> Result<Value, ZoteroError> {
        let url = self.build_url("items")?;
        self.handle_response(url).await
    }

    pub async fn fulltext_item(&self, item_key: &str) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("items/{}/fulltext", item_key))?;
        self.handle_response(url).await
    }

    pub async fn new_fulltext(&self, since: &str) -> Result<Value, ZoteroError> {
        let mut url = self.build_url("fulltext")?;
        url.query_pairs_mut().append_pair("since", since);
        self.handle_response(url).await
    }

    pub async fn trash(&self) -> Result<Value, ZoteroError> {
        let url = self.build_url("items/trash")?;
        self.handle_response(url).await
    }

    pub async fn deleted(&self, since: &str) -> Result<Value, ZoteroError> {
        let mut url = self.build_url("deleted")?;
        url.query_pairs_mut().append_pair("since", since);
        self.handle_response(url).await
    }

    pub async fn children(&self, item_id: &str) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("items/{}/children", item_id))?;
        self.handle_response(url).await
    }

    pub async fn tags(&self) -> Result<Value, ZoteroError> {
        let url = self.build_url("tags")?;
        self.handle_response(url).await
    }

    pub async fn item_tags(&self, item_id: &str) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("items/{}/tags", item_id))?;
        self.handle_response(url).await
    }
}
