use crate::{
	add_event::materialize::none,
	structured_fields::{
		StructuredEntity, StructuredFields, StructuredRelation, StructuredRelationObject,
	},
};

#[test]
fn structured_requires_update_rejects_empty_structured_fields() {
	assert!(!none::structured_requires_update(&StructuredFields::default()));
}

#[test]
fn structured_requires_update_accepts_graph_only_fields() {
	let structured = StructuredFields {
		relations: Some(vec![StructuredRelation {
			object: Some(StructuredRelationObject {
				entity: Some(StructuredEntity {
					canonical: Some("Entity".to_string()),
					..StructuredEntity::default()
				}),
				..StructuredRelationObject::default()
			}),
			..StructuredRelation::default()
		}]),
		..StructuredFields::default()
	};

	assert!(none::structured_requires_update(&structured));
}
