use serde_json::Value;

use crate::{CorpusText, LiveCaptureAction, Result, eyre};
use elf_domain::writegate::{self, WritePolicy};

pub(crate) fn elf_stored_corpus_texts(corpus: &[CorpusText]) -> Result<Vec<CorpusText>> {
	let mut stored = Vec::new();

	for item in corpus {
		if item.capture.action == LiveCaptureAction::Exclude {
			continue;
		}

		stored.push(CorpusText {
			evidence_id: item.evidence_id.clone(),
			text: transformed_capture_text(item)?.trim().to_string(),
			capture: item.capture.clone(),
		});
	}

	Ok(stored)
}

pub(crate) fn write_policy_from_value(value: &Value, evidence_id: &str) -> Result<WritePolicy> {
	serde_json::from_value::<WritePolicy>(value.clone()).map_err(|err| {
		eyre::eyre!("Failed to parse write_policy for evidence {evidence_id}: {err}")
	})
}

pub(crate) fn capture_action_str(action: LiveCaptureAction) -> &'static str {
	match action {
		LiveCaptureAction::Store => "store",
		LiveCaptureAction::Exclude => "exclude",
	}
}

fn transformed_capture_text(item: &CorpusText) -> Result<String> {
	let Some(policy_value) = &item.capture.write_policy else {
		return Ok(item.text.clone());
	};
	let policy = write_policy_from_value(policy_value, item.evidence_id.as_str())?;
	let result =
		writegate::apply_write_policy(item.text.as_str(), Some(&policy)).map_err(|err| {
			eyre::eyre!("Invalid write_policy for evidence {}: {err:?}", item.evidence_id)
		})?;

	Ok(result.transformed)
}
