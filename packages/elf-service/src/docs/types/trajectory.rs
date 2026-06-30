use serde_json::Value;

use crate::docs::{
	DocRetrievalTrajectory, DocRetrievalTrajectoryStage,
	types::constants::DOC_RETRIEVAL_TRAJECTORY_SCHEMA_V1,
};

pub(in crate::docs) struct DocTrajectoryBuilder {
	pub(in crate::docs) explain: bool,
	pub(in crate::docs) stages: Vec<DocRetrievalTrajectoryStage>,
	pub(in crate::docs) stage_order: u32,
}
impl DocTrajectoryBuilder {
	pub(in crate::docs) fn new(explain: bool) -> Self {
		Self { explain, stages: Vec::new(), stage_order: 0 }
	}

	pub(in crate::docs) fn push(&mut self, stage_name: &str, stats: Value) {
		if !self.explain {
			return;
		}

		self.stages.push(DocRetrievalTrajectoryStage {
			stage_order: self.stage_order,
			stage_name: stage_name.to_string(),
			stats,
		});

		self.stage_order += 1;
	}

	pub(in crate::docs) fn into_trajectory(self) -> Option<DocRetrievalTrajectory> {
		if !self.explain {
			return None;
		}

		Some(DocRetrievalTrajectory {
			schema: DOC_RETRIEVAL_TRAJECTORY_SCHEMA_V1.to_string(),
			stages: self.stages,
		})
	}
}
