use crate::{
	CaptureMaterializationEvidence, CaptureRuntimeEvidence, CaptureRuntimeEvidenceItem, CorpusText,
	LiveCaptureAction, capture::policy,
};

#[derive(Default)]
struct CaptureRuntimeValidator {
	failures: Vec<String>,
	expected_redactions: usize,
	expected_exclusions: usize,
}
impl CaptureRuntimeValidator {
	fn validate_corpus_item(
		&mut self,
		item: &CorpusText,
		capture: &CaptureMaterializationEvidence,
		runtime: &CaptureRuntimeEvidence,
	) {
		match item.capture.action {
			LiveCaptureAction::Exclude => self.validate_excluded_item(item, capture, runtime),
			LiveCaptureAction::Store => self.validate_stored_item(item, runtime),
		}
	}

	fn validate_excluded_item(
		&mut self,
		item: &CorpusText,
		capture: &CaptureMaterializationEvidence,
		runtime: &CaptureRuntimeEvidence,
	) {
		if runtime.item_for(item.evidence_id.as_str()).is_some() {
			self.failures.push(format!(
				"excluded evidence {} was returned by live search",
				item.evidence_id
			));
		}
		if capture.stored_evidence_ids.iter().any(|id| id == &item.evidence_id) {
			self.failures.push(format!(
				"excluded evidence {} was stored by live ingestion",
				item.evidence_id
			));
		}
		if !capture.excluded_evidence_ids.iter().any(|id| id == &item.evidence_id) {
			self.failures.push(format!(
				"excluded evidence {} was not recorded as excluded",
				item.evidence_id
			));
		}
	}

	fn validate_stored_item(&mut self, item: &CorpusText, runtime: &CaptureRuntimeEvidence) {
		let runtime_item = runtime.item_for(item.evidence_id.as_str());

		self.validate_source_id(item, runtime_item);
		self.validate_evidence_binding(item, runtime_item);
		self.validate_write_policy(item, runtime_item);
		self.validate_capture_action(item, runtime_item);
	}

	fn validate_source_id(
		&mut self,
		item: &CorpusText,
		runtime_item: Option<&CaptureRuntimeEvidenceItem>,
	) {
		let Some(expected_source_id) = item.capture.source_id.as_deref() else {
			return;
		};

		match runtime_item.and_then(|observed| observed.source_id.as_deref()) {
			Some(observed) if observed == expected_source_id => {},
			Some(observed) => self.failures.push(format!(
				"evidence {} returned source_id {observed}, expected {expected_source_id}",
				item.evidence_id
			)),
			None => self.failures.push(format!(
				"evidence {} did not return expected source_id {expected_source_id}",
				item.evidence_id
			)),
		}
	}

	fn validate_evidence_binding(
		&mut self,
		item: &CorpusText,
		runtime_item: Option<&CaptureRuntimeEvidenceItem>,
	) {
		let Some(expected_binding) = item.capture.evidence_binding.as_deref() else {
			return;
		};

		match runtime_item.and_then(|observed| observed.evidence_binding.as_deref()) {
			Some(observed) if observed == expected_binding => {},
			Some(observed) => self.failures.push(format!(
				"evidence {} returned evidence_binding {observed}, expected {expected_binding}",
				item.evidence_id
			)),
			None => self.failures.push(format!(
				"evidence {} did not return expected evidence_binding {expected_binding}",
				item.evidence_id
			)),
		}
	}

	fn validate_write_policy(
		&mut self,
		item: &CorpusText,
		runtime_item: Option<&CaptureRuntimeEvidenceItem>,
	) {
		let Some(policy_value) = &item.capture.write_policy else {
			return;
		};

		match policy::write_policy_from_value(policy_value, item.evidence_id.as_str()) {
			Ok(policy) => {
				self.expected_exclusions += policy.exclusions.len();
				self.expected_redactions += policy.redactions.len();
			},
			Err(err) => self.failures.push(err.to_string()),
		}

		if !runtime_item.is_some_and(|observed| observed.write_policy_applied) {
			self.failures.push(format!(
				"evidence {} did not return write_policy_applied=true",
				item.evidence_id
			));
		}
	}

	fn validate_capture_action(
		&mut self,
		item: &CorpusText,
		runtime_item: Option<&CaptureRuntimeEvidenceItem>,
	) {
		if let Some(observed) = runtime_item.and_then(|observed| observed.capture_action.as_deref())
			&& observed != policy::capture_action_str(item.capture.action)
		{
			self.failures.push(format!(
				"evidence {} returned capture_action {observed}, expected {}",
				item.evidence_id,
				policy::capture_action_str(item.capture.action)
			));
		}
	}

	fn validate_write_policy_totals(
		mut self,
		capture: &CaptureMaterializationEvidence,
	) -> Option<String> {
		if capture.write_policy_exclusion_count < self.expected_exclusions {
			self.failures.push(format!(
				"write-policy exclusion count {} was below expected {}",
				capture.write_policy_exclusion_count, self.expected_exclusions
			));
		}
		if capture.write_policy_redaction_count < self.expected_redactions {
			self.failures.push(format!(
				"write-policy redaction count {} was below expected {}",
				capture.write_policy_redaction_count, self.expected_redactions
			));
		}
		if self.expected_exclusions + self.expected_redactions > 0
			&& capture.write_policy_audit_count == 0
		{
			self.failures.push(
				"write-policy audit count was zero despite expected policy effects".to_string(),
			);
		}
		if self.failures.is_empty() {
			None
		} else {
			Some(format!("Capture runtime validation failed: {}", self.failures.join("; ")))
		}
	}
}

pub(crate) fn validate_capture_runtime_evidence(
	suite: &str,
	corpus: &[CorpusText],
	capture: &CaptureMaterializationEvidence,
	runtime: &CaptureRuntimeEvidence,
) -> Option<String> {
	if suite != "capture_integration" {
		return None;
	}

	let mut validator = CaptureRuntimeValidator::default();

	for item in corpus {
		validator.validate_corpus_item(item, capture, runtime);
	}

	validator.validate_write_policy_totals(capture)
}
