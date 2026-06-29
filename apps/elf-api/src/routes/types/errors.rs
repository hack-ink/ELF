use super::*;

#[derive(Debug, Serialize, ToSchema)]
pub(in crate::routes) struct ErrorBody {
	pub(in crate::routes) error_code: String,
	pub(in crate::routes) message: String,
	pub(in crate::routes) fields: Option<Vec<String>>,
}
