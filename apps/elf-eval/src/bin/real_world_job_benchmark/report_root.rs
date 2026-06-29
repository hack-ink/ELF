use super::*;

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct RealWorldReport {
	pub(super) schema: String,
	pub(super) run_id: String,
	pub(super) generated_at: String,
	pub(super) runner_version: String,
	pub(super) corpus_profile: String,
	pub(super) adapter: AdapterReport,
	#[serde(default)]
	pub(super) scoreboard: ScoreboardReport,
	#[serde(default)]
	pub(super) operational_evidence: OperationalEvidenceReport,
	#[serde(default)]
	pub(super) external_adapters: ExternalAdapterSection,
	pub(super) capture_integration: CaptureIntegrationReport,
	pub(super) summary: ReportSummary,
	pub(super) suites: Vec<SuiteReport>,
	pub(super) jobs: Vec<JobReport>,
	pub(super) unsupported_claims: Vec<UnsupportedClaimReport>,
	pub(super) not_encoded_suites: Vec<String>,
	pub(super) private_corpus_redaction: PrivateCorpusRedaction,
	#[serde(default)]
	pub(super) evolution: EvolutionSummary,
	#[serde(default)]
	pub(super) follow_ups: Vec<FollowUpReport>,
}
