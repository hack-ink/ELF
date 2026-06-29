//! Structured-field validation and persistence helpers.

mod persistence;
mod types;
mod validation;

pub use self::{
	persistence::{fetch_structured_fields, upsert_structured_fields_tx},
	types::{StructuredEntity, StructuredFields, StructuredRelation, StructuredRelationObject},
	validation::{event_evidence_quotes, validate_structured_fields},
};

#[cfg(test)] mod tests;
