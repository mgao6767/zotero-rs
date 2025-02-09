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
}
