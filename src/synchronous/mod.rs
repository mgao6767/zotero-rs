use bytes::Bytes;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::Url;
use serde_json::Value;
use std::vec::IntoIter;
use thiserror::Error;

use crate::errors::ZoteroError;
use crate::{API_VERSION, VERSION};

#[derive(Debug)]
pub struct Zotero {
    client: Client,
    api_key: String,
    endpoint: String,
    pub library_id: String,
    pub library_type: String,
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

    pub fn new(
        library_id: String,
        library_type: String,
        api_key: String,
    ) -> Result<Self, ZoteroError> {
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

    fn build_url(&self, path: &str, params: Option<&[(&str, &str)]>) -> Result<Url, ZoteroError> {
        let mut url = Url::parse(&format!(
            "{}/{}/{}/{}",
            self.endpoint, self.library_type, self.library_id, path
        ))?;
        if let Some(ref loc) = self.locale {
            url.query_pairs_mut().append_pair("locale", loc);
        }
        if let Some(params) = params {
            let mut pairs = url.query_pairs_mut();
            for &(key, value) in params {
                pairs.append_pair(key, value);
            }
        }
        Ok(url)
    }

    fn build_url_no_lib(
        &self,
        path: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Url, ZoteroError> {
        let mut url = Url::parse(&format!("{}/{}", self.endpoint, path))?;
        if let Some(ref loc) = self.locale {
            url.query_pairs_mut().append_pair("locale", loc);
        }
        if let Some(params) = params {
            let mut pairs = url.query_pairs_mut();
            for &(key, value) in params {
                pairs.append_pair(key, value);
            }
        }
        Ok(url)
    }

    fn handle_response(&self, url: Url) -> Result<Value, ZoteroError> {
        let mut attempts = 0;
        let mut backoff = 0.0;
        while attempts < self.max_retries {
            let response = self
                .client
                .get(url.clone())
                .headers(self.default_headers()?)
                .send()?;

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
                std::thread::sleep(std::time::Duration::from_secs_f64(backoff));
                attempts += 1;
                continue;
            }

            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if content_type.starts_with("application/json") {
                let json: Value = response.json()?;
                return Ok(json);
            } else if content_type.starts_with("text/html") {
                let text = response.text()?;
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

    pub fn get_key_info(&self, params: Option<&[(&str, &str)]>) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("keys/{}", self.api_key), params)?;
        self.handle_response(url)
    }

    pub fn get_top(&self, params: Option<&[(&str, &str)]>) -> Result<Value, ZoteroError> {
        let url = self.build_url("items/top", params)?;
        self.handle_response(url)
    }

    pub fn get_collections(&self, params: Option<&[(&str, &str)]>) -> Result<Value, ZoteroError> {
        let url = self.build_url("collections", params)?;
        self.handle_response(url)
    }

    pub fn get_collection(
        &self,
        collection_id: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("collections/{}", collection_id), params)?;
        self.handle_response(url)
    }

    pub fn get_collections_top(
        &self,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ZoteroError> {
        let url = self.build_url("collections/top", params)?;
        self.handle_response(url)
    }

    pub fn get_collections_sub(
        &self,
        collection_id: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ZoteroError> {
        let url = self.build_url(
            &format!("collections/{}/collections", collection_id),
            params,
        )?;
        self.handle_response(url)
    }

    pub fn get_collection_items(
        &self,
        collection_id: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("collections/{}/items", collection_id), params)?;
        self.handle_response(url)
    }

    pub fn get_item(
        &self,
        item_id: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("items/{}", item_id), params)?;
        self.handle_response(url)
    }

    pub fn get_items(&self, params: Option<&[(&str, &str)]>) -> Result<Value, ZoteroError> {
        let url = self.build_url("items", params)?;
        self.handle_response(url)
    }

    pub fn get_fulltext_item(
        &self,
        item_key: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("items/{}/fulltext", item_key), params)?;
        self.handle_response(url)
    }

    pub fn get_new_fulltext(
        &self,
        since: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ZoteroError> {
        let mut url = self.build_url("fulltext", params)?;
        url.query_pairs_mut().append_pair("since", since);
        self.handle_response(url)
    }

    pub fn get_trash(&self, params: Option<&[(&str, &str)]>) -> Result<Value, ZoteroError> {
        let url = self.build_url("items/trash", params)?;
        self.handle_response(url)
    }

    pub fn get_deleted(
        &self,
        since: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ZoteroError> {
        let mut url = self.build_url("deleted", params)?;
        url.query_pairs_mut().append_pair("since", since);
        self.handle_response(url)
    }

    pub fn get_children(
        &self,
        item_id: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("items/{}/children", item_id), params)?;
        self.handle_response(url)
    }

    pub fn get_tags(&self, params: Option<&[(&str, &str)]>) -> Result<Value, ZoteroError> {
        let url = self.build_url("tags", params)?;
        self.handle_response(url)
    }

    pub fn get_item_tags(
        &self,
        item_id: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Value, ZoteroError> {
        let url = self.build_url(&format!("items/{}/tags", item_id), params)?;
        self.handle_response(url)
    }

    pub fn get_file(
        &self,
        item_id: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Bytes, ZoteroError> {
        let url = self.build_url(&format!("items/{}/file", item_id), params)?;
        let response = self
            .client
            .get(url)
            .headers(self.default_headers()?)
            .send()?;

        if response.status().is_success() {
            let bytes = response.bytes()?;
            Ok(bytes)
        } else {
            Err(ZoteroError::FileRetrievalError(format!(
                "Failed to retrieve file: {}",
                response.status()
            )))
        }
    }

    pub fn get_last_modified_version(
        &self,
        params: Option<&[(&str, &str)]>,
    ) -> Result<i64, ZoteroError> {
        let mut params_with_limit = params.unwrap_or(&[]).to_vec();
        params_with_limit.push(("limit", "1"));
        let url = self.build_url("items", Some(params_with_limit.as_slice()))?;
        let response = self
            .client
            .get(url)
            .headers(self.default_headers()?)
            .send()?;

        if response.status().is_success() {
            if let Some(last_modified_version) = response.headers().get("last-modified-version") {
                if let Ok(version_str) = last_modified_version.to_str() {
                    if let Ok(version) = version_str.parse::<i64>() {
                        return Ok(version);
                    }
                }
            }
            Err(ZoteroError::FileRetrievalError(
                "Failed to parse last-modified-version header".to_string(),
            ))
        } else {
            Err(ZoteroError::FileRetrievalError(format!(
                "Failed to retrieve last modified version: {}",
                response.status()
            )))
        }
    }

    pub fn get_item_types(&self) -> Result<Value, ZoteroError> {
        let url = self.build_url_no_lib("itemTypes", None)?;
        self.handle_response(url)
    }

    pub fn get_item_fields(&self) -> Result<Value, ZoteroError> {
        let url = self.build_url_no_lib("itemFields", None)?;
        self.handle_response(url)
    }

    pub fn get_creator_fields(&self) -> Result<Value, ZoteroError> {
        let url = self.build_url_no_lib("creatorFields", None)?;
        self.handle_response(url)
    }

    pub fn get_item_type_fields(&self, item_type: &str) -> Result<Value, ZoteroError> {
        let url = self.build_url_no_lib("itemTypeFields", Some(&[("itemType", item_type)]))?;
        self.handle_response(url)
    }

    pub fn get_item_creator_types(&self, item_type: &str) -> Result<Value, ZoteroError> {
        let url =
            self.build_url_no_lib("itemTypeCreatorTypes", Some(&[("itemType", item_type)]))?;
        self.handle_response(url)
    }

    pub fn get_items_in_batch(&self, since: usize, batch_size: usize) -> ZoteroItemsBatcher {
        ZoteroItemsBatcher::new(self, since, batch_size, false)
    }

    pub fn get_trashed_items_in_batch(
        &self,
        since: usize,
        batch_size: usize,
    ) -> ZoteroItemsBatcher {
        ZoteroItemsBatcher::new(self, since, batch_size, true)
    }

    pub fn get_collections_in_batch(&self, batch_size: usize) -> ZoteroCollectionBatcher {
        ZoteroCollectionBatcher::new(self, batch_size)
    }
}

#[derive(Error, Debug)]
pub enum ZoteroBatchError {
    #[error("No more items to fetch")]
    NoMoreItems,
    #[error("Failed to fetch items: {0}")]
    FetchError(#[from] Box<dyn std::error::Error>),
}

pub struct ZoteroItemsBatcher<'a> {
    zotero: &'a Zotero,
    since: usize,
    start: usize,
    limit: usize,
    items: IntoIter<Value>,
    trash: bool,
}

impl<'a> ZoteroItemsBatcher<'a> {
    fn new(zotero: &'a Zotero, since: usize, batch_size: usize, trash: bool) -> Self {
        Self {
            zotero,
            since,
            start: 0,
            limit: batch_size,
            items: vec![].into_iter(),
            trash: trash,
        }
    }

    fn fetch_next_batch(&mut self) -> Result<(), ZoteroBatchError> {
        println!("Fetching batch starting at {}", self.start);
        let response = match self.trash {
            true => self
                .zotero
                .get_trash(Some(&[
                    ("start", &self.start.to_string()),
                    ("since", &self.since.to_string()),
                    ("limit", &self.limit.to_string()),
                    ("sort", "dateAdded"),
                    ("direction", "asc"),
                ]))
                .map_err(|e| ZoteroBatchError::FetchError(Box::new(e)))?,
            false => self
                .zotero
                .get_items(Some(&[
                    ("start", &self.start.to_string()),
                    ("since", &self.since.to_string()),
                    ("limit", &self.limit.to_string()),
                    ("sort", "dateAdded"),
                    ("direction", "asc"),
                ]))
                .map_err(|e| ZoteroBatchError::FetchError(Box::new(e)))?,
        };
        let items = response.as_array().unwrap_or(&vec![]).clone();
        if items.is_empty() {
            return Err(ZoteroBatchError::NoMoreItems);
        }
        self.items = items.into_iter();
        Ok(())
    }
}

impl<'a> Iterator for ZoteroItemsBatcher<'a> {
    type Item = Result<Value, ZoteroBatchError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.items.next() {
            self.start += 1;
            Some(Ok(item))
        } else {
            match self.fetch_next_batch() {
                Ok(_) => self.next(),
                Err(ZoteroBatchError::NoMoreItems) => None,
                Err(e) => Some(Err(e)),
            }
        }
    }
}

pub struct ZoteroCollectionBatcher<'a> {
    zotero: &'a Zotero,
    start: usize,
    limit: usize,
    collections: IntoIter<Value>,
}

impl<'a> ZoteroCollectionBatcher<'a> {
    fn new(zotero: &'a Zotero, batch_size: usize) -> Self {
        Self {
            zotero,
            start: 0,
            limit: batch_size,
            collections: vec![].into_iter(),
        }
    }

    fn fetch_next_batch(&mut self) -> Result<(), ZoteroBatchError> {
        println!("Fetching collections batch starting at {}", self.start);
        let response = self
            .zotero
            .get_collections(Some(&[
                ("start", &self.start.to_string()),
                ("limit", &self.limit.to_string()),
            ]))
            .map_err(|e| ZoteroBatchError::FetchError(Box::new(e)))?;
        let collections = response.as_array().unwrap_or(&vec![]).clone();
        if collections.is_empty() {
            return Err(ZoteroBatchError::NoMoreItems);
        }
        self.collections = collections.into_iter();
        Ok(())
    }
}

impl<'a> Iterator for ZoteroCollectionBatcher<'a> {
    type Item = Result<Value, ZoteroBatchError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(collection) = self.collections.next() {
            self.start += 1;
            Some(Ok(collection))
        } else {
            match self.fetch_next_batch() {
                Ok(_) => self.next(),
                Err(ZoteroBatchError::NoMoreItems) => None,
                Err(e) => Some(Err(e)),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Zotero;
    use dotenv::dotenv;
    use std::env;

    #[test]
    fn test() {
        dotenv().ok();
        let api_key = env::var("ZOTERO_API_KEY").expect("ZOTERO_API_KEY not found");
        let lib_id = env::var("ZOTERO_LIBRARY_ID").expect("ZOTERO_LIBRARY_ID not found");
        let zotero = Zotero::group_lib(&lib_id, &api_key).unwrap();
        for result in zotero.get_collections_in_batch(100) {
            match result {
                Ok(c) => {
                    println!("{:?}", c);
                }
                Err(e) => {
                    eprintln!("Error fetching collection: {}", e);
                }
            }
        }
    }
}
