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

Inside a sync function, use `tokio::runtime::Runtime`:

```rust
use zotero_rs::Zotero;

fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let zotero = Zotero::group_lib("library_id", "api_key").unwrap();
    match runtime.block_on(zotero.items(None)) {
        Ok(items) => println!("{:#?}", items),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```
