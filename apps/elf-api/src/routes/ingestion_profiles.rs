mod collection;
mod defaults;
mod versions;

pub(super) use self::{
	collection::{
		__path_admin_ingestion_profile_create, __path_admin_ingestion_profiles_list,
		admin_ingestion_profile_create, admin_ingestion_profiles_list,
	},
	defaults::{
		__path_admin_ingestion_profile_default_get, __path_admin_ingestion_profile_default_set,
		admin_ingestion_profile_default_get, admin_ingestion_profile_default_set,
	},
	versions::{
		__path_admin_ingestion_profile_get, __path_admin_ingestion_profile_versions_list,
		admin_ingestion_profile_get, admin_ingestion_profile_versions_list,
	},
};
