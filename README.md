# zotero-rs

A Rust library for interacting with the Zotero API, providing both synchronous and asynchronous versions of the `Zotero` struct.

## Installation

```bash
cargo add zotero-rs
```

## Usage

### Synchronous Example

Get collections info.

```rust
use zotero_rs::Zotero;

fn main() {
    let zotero = Zotero::user_lib("your_user_id", "your_api_key").unwrap();
    let collections = zotero.get_collections(None).unwrap();
    println!("{:?}", collections);
}
```

Get items in batches. This will allow iterating over **all** items since a given version. The library will automatically fetch items in batches until no more items are available.

```rust
use dotenv::dotenv;
use std::env;
use zotero_rs::Zotero;

fn main() {
    dotenv().ok();
    let api_key = env::var("ZOTERO_API_KEY").expect("ZOTERO_API_KEY not found");
    let lib_id = env::var("ZOTERO_LIBRARY_ID").expect("ZOTERO_LIBRARY_ID not found");
    let zotero = Zotero::group_lib(&lib_id, &api_key).unwrap();
    for item in zotero.get_items_in_batch(0, 100) {
        match item {
            Ok(value) => println!("Item: {:?}", value),
            Err(e) => println!("Error: {:?}", e),
        }
    }
}
```

### Asynchronous Example

```rust
use zotero_rs::ZoteroAsync as Zotero;

#[tokio::main]
async fn main() {
    let zotero = Zotero::user_lib("your_user_id", "your_api_key").await.unwrap();
    let collections = zotero.get_collections(None).await.unwrap();
    println!("{:?}", collections);
}
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or bug fixes.

## License

This project is licensed under the MIT License. See the LICENSE file for details.