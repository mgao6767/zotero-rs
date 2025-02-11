mod asynchronous;
mod synchronous;

pub mod errors;
pub use errors::ZoteroError;

pub use asynchronous::Zotero as ZoteroAsync;
pub use synchronous::Zotero;

const VERSION: &str = "1";
const API_VERSION: &str = "3";
