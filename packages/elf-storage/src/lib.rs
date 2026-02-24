pub mod db;
pub mod doc_outbox;
pub mod docs;
pub mod graph;
pub mod models;
pub mod outbox;
pub mod qdrant;
pub mod queries;
pub mod schema;

mod error;

pub use error::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;
