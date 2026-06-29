use crate::search::{
	Deserialize, Duration, ExpansionMode, OffsetDateTime, SearchExplain, Serialize, TRACE_VERSION,
	Uuid, Value, ranking,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::search) struct TracePayload {
	pub(in crate::search) trace: TraceRecord,
	pub(in crate::search) items: Vec<TraceItemRecord>,
	#[serde(default)]
	pub(in crate::search) candidates: Vec<TraceCandidateRecord>,
	#[serde(default)]
	pub(in crate::search) stages: Vec<TraceTrajectoryStageRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::search) struct TraceRecord {
	pub(in crate::search) trace_id: Uuid,
	pub(in crate::search) tenant_id: String,
	pub(in crate::search) project_id: String,
	pub(in crate::search) agent_id: String,
	pub(in crate::search) read_profile: String,
	pub(in crate::search) query: String,
	pub(in crate::search) expansion_mode: String,
	pub(in crate::search) expanded_queries: Vec<String>,
	pub(in crate::search) allowed_scopes: Vec<String>,
	pub(in crate::search) candidate_count: u32,
	pub(in crate::search) top_k: u32,
	pub(in crate::search) config_snapshot: Value,
	pub(in crate::search) trace_version: i32,
	pub(in crate::search) created_at: OffsetDateTime,
	pub(in crate::search) expires_at: OffsetDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::search) struct TraceItemRecord {
	pub(in crate::search) item_id: Uuid,
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) chunk_id: Option<Uuid>,
	pub(in crate::search) rank: u32,
	pub(in crate::search) final_score: f32,
	pub(in crate::search) explain: SearchExplain,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::search) struct TraceCandidateRecord {
	pub(in crate::search) candidate_id: Uuid,
	pub(in crate::search) note_id: Uuid,
	pub(in crate::search) chunk_id: Uuid,
	pub(in crate::search) chunk_index: i32,
	pub(in crate::search) snippet: String,
	#[serde(default)]
	pub(in crate::search) candidate_snapshot: Value,
	pub(in crate::search) retrieval_rank: u32,
	pub(in crate::search) rerank_score: f32,
	pub(in crate::search) note_scope: String,
	pub(in crate::search) note_importance: f32,
	pub(in crate::search) note_updated_at: OffsetDateTime,
	pub(in crate::search) note_hit_count: i64,
	pub(in crate::search) note_last_hit_at: Option<OffsetDateTime>,
	pub(in crate::search) created_at: OffsetDateTime,
	pub(in crate::search) expires_at: OffsetDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::search) struct TraceTrajectoryStageRecord {
	pub(in crate::search) stage_id: Uuid,
	pub(in crate::search) stage_order: u32,
	pub(in crate::search) stage_name: String,
	pub(in crate::search) stage_payload: Value,
	pub(in crate::search) created_at: OffsetDateTime,
	#[serde(default)]
	pub(in crate::search) items: Vec<TraceTrajectoryStageItemRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::search) struct TraceTrajectoryStageItemRecord {
	pub(in crate::search) id: Uuid,
	pub(in crate::search) item_id: Option<Uuid>,
	pub(in crate::search) note_id: Option<Uuid>,
	pub(in crate::search) chunk_id: Option<Uuid>,
	pub(in crate::search) metrics: Value,
}

pub(in crate::search) struct TraceContext<'a> {
	pub(in crate::search) trace_id: Uuid,
	pub(in crate::search) tenant_id: &'a str,
	pub(in crate::search) project_id: &'a str,
	pub(in crate::search) agent_id: &'a str,
	pub(in crate::search) read_profile: &'a str,
	pub(in crate::search) query: &'a str,
	pub(in crate::search) expansion_mode: ExpansionMode,
	pub(in crate::search) expanded_queries: Vec<String>,
	pub(in crate::search) allowed_scopes: &'a [String],
	pub(in crate::search) candidate_count: usize,
	pub(in crate::search) top_k: u32,
}

pub(in crate::search) struct SearchTraceBuilder {
	pub(in crate::search) trace: TraceRecord,
	pub(in crate::search) items: Vec<TraceItemRecord>,
	pub(in crate::search) candidates: Vec<TraceCandidateRecord>,
	pub(in crate::search) stages: Vec<TraceTrajectoryStageRecord>,
}
impl SearchTraceBuilder {
	pub(in crate::search) fn new(
		context: TraceContext<'_>,
		config_snapshot: Value,
		retention_days: i64,
		now: OffsetDateTime,
	) -> Self {
		let trace = TraceRecord {
			trace_id: context.trace_id,
			tenant_id: context.tenant_id.to_string(),
			project_id: context.project_id.to_string(),
			agent_id: context.agent_id.to_string(),
			read_profile: context.read_profile.to_string(),
			query: context.query.to_string(),
			expansion_mode: ranking::expansion_mode_label(context.expansion_mode).to_string(),
			expanded_queries: context.expanded_queries,
			allowed_scopes: context.allowed_scopes.to_vec(),
			candidate_count: context.candidate_count as u32,
			top_k: context.top_k,
			config_snapshot,
			trace_version: TRACE_VERSION,
			created_at: now,
			expires_at: now + Duration::days(retention_days),
		};

		Self { trace, items: Vec::new(), candidates: Vec::new(), stages: Vec::new() }
	}

	pub(in crate::search) fn push_item(&mut self, item: TraceItemRecord) {
		self.items.push(item);
	}

	pub(in crate::search) fn push_candidate(&mut self, candidate: TraceCandidateRecord) {
		self.candidates.push(candidate);
	}

	pub(in crate::search) fn push_stage(&mut self, stage: TraceTrajectoryStageRecord) {
		self.stages.push(stage);
	}

	pub(in crate::search) fn build(self) -> TracePayload {
		TracePayload {
			trace: self.trace,
			items: self.items,
			candidates: self.candidates,
			stages: self.stages,
		}
	}
}
