# zotero-rs

A Rust library for interacting with the Zotero API.

```rust
use zotero_rs::Zotero;

#[tokio::main]
async fn main() {
    let zotero = Zotero::group_lib("library_id", "api_key").unwrap();
    match zotero.top(None).await {
        Ok(items) => println!("{:#?}", items),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```
