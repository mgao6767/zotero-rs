#[cfg(test)]
mod mock_tests {
    use httpmock::prelude::*;
    use std::fs;
    use tokio;
    use zotero_rs::{Zotero, ZoteroError};

    #[tokio::test]
    async fn test_get_items() {
        let server = MockServer::start();
        let items_doc = fs::read_to_string("tests/api_responses/items_doc.json")
            .expect("Failed to read items_doc.json");
        let mock = server.mock(|when, then| {
            when.method(GET).path("/users/myuserID/items");
            then.status(200)
                .header("content-type", "application/json")
                .body(&items_doc);
        });
        let mut zot = Zotero::user_lib("myuserID", "myuserkey").unwrap();
        zot.set_endpoint(&server.base_url());
        let items = zot.items().await.unwrap();
        println!("{:?}", items);
        mock.assert();
    }

    #[tokio::test]
    async fn test_parse_item_json_doc() {
        let server = MockServer::start();
        let item_doc = std::fs::read_to_string("tests/api_responses/item_doc.json")
            .expect("Failed to read item_doc.json");
        server.mock(|when, then| {
            when.method(GET).path("/users/myuserID/items");
            then.status(200)
                .header("content-type", "application/json")
                .body(&item_doc);
        });
        let mut zot = Zotero::user_lib("myuserID", "myuserkey").unwrap();
        zot.set_endpoint(&server.base_url());
        let items_data = zot.items().await.unwrap();
        let key = items_data["data"]["key"].as_str().unwrap();
        assert_eq!(key, "X42A7DEE");
        let name = items_data["data"]["creators"][0]["name"].as_str().unwrap();
        assert_eq!(name, "Institute of Physics (Great Britain)");
        let item_type = items_data["data"]["itemType"].as_str().unwrap();
        assert_eq!(item_type, "book");
        let date_modified = items_data["data"]["dateModified"].as_str().unwrap();
        let test_dt = chrono::DateTime::parse_from_rfc3339("2011-01-13T03:37:29Z").unwrap();
        let incoming_dt = chrono::DateTime::parse_from_rfc3339(date_modified).unwrap();
        assert_eq!(test_dt, incoming_dt);
    }

    #[tokio::test]
    async fn test_locale() {
        let server = MockServer::start();
        let item_doc = std::fs::read_to_string("tests/api_responses/item_doc.json")
            .expect("Failed to read item_doc.json");
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/users/myuserID/items")
                .query_param("locale", "en-US");
            then.status(200)
                .header("content-type", "application/json")
                .body(&item_doc);
        });

        let mut zot = Zotero::user_lib("myuserID", "myuserkey").unwrap();
        zot.set_locale("en-US");
        zot.set_endpoint(&server.base_url());
        let items_data = zot.items().await.unwrap();
        // Verify something in the JSON
        let key = items_data["data"]["key"].as_str().unwrap();
        assert_eq!(key, "X42A7DEE");
        mock.assert();
    }

    #[tokio::test]
    async fn test_backoff() {
        let server = MockServer::start();
        let item_doc = std::fs::read_to_string("tests/api_responses/item_doc.json")
            .expect("Failed to read item_doc.json");

        server.mock(|when, then| {
            when.method(GET).path("/users/myuserID/items");
            then.status(429)
                .header("content-type", "application/json")
                .header("backoff", "0.2")
                .body(&item_doc);
        });

        let mut zot = Zotero::user_lib("myuserID", "myuserkey").unwrap();
        zot.set_endpoint(&server.base_url());
        let future = zot.items();

        let result = future.await;
        // Assert that the error is TooManyRequests
        assert!(matches!(result, Err(ZoteroError::TooManyRequests(_))));
    }

    #[tokio::test]
    async fn test_get_file() {
        let server = MockServer::start();
        let file_content =
            fs::read("tests/api_responses/item_file.pdf").expect("Failed to read item_file.pdf");
        let mock = server.mock(|when, then| {
            when.method(GET).path("/users/myuserID/items/MYITEMID/file");
            then.status(200)
                .header("content-type", "application/pdf")
                .body(file_content.clone());
        });

        let mut zot = Zotero::user_lib("myuserID", "myuserkey").unwrap();
        zot.set_endpoint(&server.base_url());
        let file_data = zot.file("MYITEMID").await.unwrap();
        assert_eq!(file_data, file_content);
        let expected_data = b"One very strange PDF\n";
        assert_eq!(&file_data[..expected_data.len()], expected_data);
        mock.assert();
    }
}
