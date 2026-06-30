mod excerpts;
mod read;
mod search_l0;
mod write;

pub(super) use self::{
	excerpts::{
		__path_admin_docs_excerpts_get, __path_docs_excerpts_get, admin_docs_excerpts_get,
		docs_excerpts_get,
	},
	read::{__path_admin_docs_get, __path_docs_get, admin_docs_get, docs_get},
	search_l0::{
		__path_admin_docs_search_l0, __path_docs_search_l0, admin_docs_search_l0, docs_search_l0,
	},
	write::{__path_docs_delete, __path_docs_put, docs_delete, docs_put},
};
