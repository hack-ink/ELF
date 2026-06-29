//! Structured-field validation and persistence helpers.

mod persistence;
#[cfg(test)] mod tests;
mod types;
mod validation;

pub use persistence::{fetch_structured_fields, upsert_structured_fields_tx};
pub use types::{StructuredEntity, StructuredFields, StructuredRelation, StructuredRelationObject};
pub use validation::{event_evidence_quotes, validate_structured_fields};
