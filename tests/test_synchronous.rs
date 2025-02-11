#[cfg(test)]
mod mock_tests {
    use httpmock::prelude::*;
    use std::fs;
    use zotero_rs::Zotero;
    use zotero_rs::ZoteroError;

    #[test]
    fn test_get_items() {
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
        let items = zot.get_items(None).unwrap();
        println!("{:?}", items);
        mock.assert();
    }

    #[test]
    fn test_parse_item_json_doc() {
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
        let items_data = zot.get_items(None).unwrap();
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

    #[test]
    fn test_locale() {
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
        let items_data = zot.get_items(None).unwrap();
        // Verify something in the JSON
        let key = items_data["data"]["key"].as_str().unwrap();
        assert_eq!(key, "X42A7DEE");
        mock.assert();
    }

    #[test]
    fn test_backoff() {
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
        let result = zot.get_items(None);
        // Assert that the error is TooManyRequests
        assert!(matches!(result, Err(ZoteroError::TooManyRequests(_))));
    }

    #[test]
    fn test_get_file() {
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
        let file_data = zot.get_file("MYITEMID", None).unwrap();
        assert_eq!(file_data, file_content);
        // Normalize line endings to LF for comparison
        let expected_data = b"One very strange PDF\n";
        let normalized_file_data: Vec<u8> = file_data
            .iter()
            .map(|&b| if b == b'\r' { b'\n' } else { b })
            .collect();
        assert_eq!(&normalized_file_data[..expected_data.len()], expected_data);
        mock.assert();
    }

    #[test]
    fn test_last_modified_version() {
        let server = MockServer::start();
        let items_doc = fs::read_to_string("tests/api_responses/items_doc.json")
            .expect("Failed to read items_doc.json");
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/users/myuserID/items")
                .query_param("limit", "1");
            then.status(200)
                .header("content-type", "application/json")
                .header("last-modified-version", "12345")
                .body(&items_doc);
        });

        let mut zot = Zotero::user_lib("myuserID", "myuserkey").unwrap();
        zot.set_endpoint(&server.base_url());
        let version = zot.get_last_modified_version(None).unwrap();
        assert_eq!(version, 12345);
        mock.assert();
    }
}
