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
}

#[derive(Debug)]
pub struct Zotero {
    client: Client,
    api_key: String,
    endpoint: String,
    library_id: String,
    library_type: String,
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
        })
    }

    pub fn set_endpoint(&mut self, endpoint: &str) {
        self.endpoint = endpoint.to_string();
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
        let base_url = format!(
            "{}/{}/{}/{}",
            self.endpoint, self.library_type, self.library_id, path
        );
        Ok(Url::parse(&base_url)?)
    }

    async fn handle_response(&self, url: Url) -> Result<Value, ZoteroError> {
        let response = self
            .client
            .get(url)
            .headers(self.default_headers()?)
            .send()
            .await?;

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.starts_with("application/json") {
            let json: Value = response.json().await?;
            Ok(json)
        } else if content_type.starts_with("text/html") {
            let text = response.text().await?;
            Ok(Value::String(text))
        } else {
            Err(ZoteroError::UnsupportedContentType(
                content_type.to_string(),
            ))
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use std::env;
    use tokio;

    struct TestFixture {
        zotero: Zotero,
    }

    impl TestFixture {
        fn new() -> Self {
            dotenv().ok();
            let library_id = env::var("ZOTERO_LIBRARY_ID").expect("ZOTERO_LIBRARY_ID not set");
            let api_key = env::var("ZOTERO_API_KEY").expect("ZOTERO_API_KEY not set");
            TestFixture {
                zotero: Zotero::group_lib(&library_id, &api_key).unwrap(),
            }
        }
    }

    #[tokio::test]
    async fn test_key_info() {
        let fixture = TestFixture::new();
        match fixture.zotero.key_info().await {
            Ok(info) => println!("{:#?}", info),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_top() {
        let fixture = TestFixture::new();
        match fixture.zotero.top().await {
            Ok(items) => println!("{:#?}", items),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_collections() {
        let fixture = TestFixture::new();
        match fixture.zotero.collections().await {
            Ok(collections) => println!("{:#?}", collections),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_collection() {
        let fixture = TestFixture::new();
        match fixture.zotero.collection("6PAX58L2").await {
            Ok(collection) => println!("{:#?}", collection),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_collections_top() {
        let fixture = TestFixture::new();
        match fixture.zotero.collections_top().await {
            Ok(collections) => println!("{:#?}", collections),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_collections_sub() {
        let fixture = TestFixture::new();
        match fixture.zotero.collections_sub("UF2SAMGA").await {
            Ok(collections) => println!("{:#?}", collections),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_collection_items() {
        let fixture = TestFixture::new();
        match fixture.zotero.collection_items("UL9B8URP").await {
            Ok(items) => println!("{:#?}", items),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_item() {
        let fixture = TestFixture::new();
        match fixture.zotero.item("FLZIXG7A").await {
            Ok(item) => println!("{:#?}", item),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_items() {
        let fixture = TestFixture::new();
        match fixture.zotero.items().await {
            Ok(items) => println!("{:#?}", items),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_fulltext_item() {
        let fixture = TestFixture::new();
        match fixture.zotero.fulltext_item("J6V8C845").await {
            Ok(fulltext) => println!("{:#?}", fulltext),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_new_fulltext() {
        let fixture = TestFixture::new();
        match fixture.zotero.new_fulltext("20000").await {
            Ok(fulltext) => println!("{:#?}", fulltext),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_trash() {
        let fixture = TestFixture::new();
        match fixture.zotero.trash().await {
            Ok(trash) => println!("{:#?}", trash),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_deleted() {
        let fixture = TestFixture::new();
        match fixture.zotero.deleted("25000").await {
            Ok(deleted) => println!("{:#?}", deleted),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_children() {
        let fixture = TestFixture::new();
        match fixture.zotero.children("FLZIXG7A").await {
            Ok(children) => println!("{:#?}", children),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_tags() {
        let fixture = TestFixture::new();
        match fixture.zotero.tags().await {
            Ok(tags) => println!("{:#?}", tags),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_item_tags() {
        let fixture = TestFixture::new();
        match fixture.zotero.item_tags("FLZIXG7A").await {
            Ok(tags) => println!("{:#?}", tags),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
