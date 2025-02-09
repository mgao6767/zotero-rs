#[cfg(test)]
mod mock_tests {
    use httpmock::prelude::*;
    use std::fs;
    use tokio;
    use zotero_rs::Zotero;

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
}
