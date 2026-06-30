use crate::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct FollowUpReport {
	pub(crate) suite_id: String,
	pub(crate) job_id: String,
	pub(crate) title: String,
	pub(crate) reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct PrivateCorpusRedaction {
	pub(crate) policy: String,
	pub(crate) private_fixture_count: usize,
}
