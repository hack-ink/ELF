mod locators;
mod metadata;
mod requirements;
mod strings;

pub(in crate::docs) use self::{
	metadata::validate_source_library_metadata, requirements::validate_doc_source_ref_requirements,
	strings::extract_source_ref_string,
};
