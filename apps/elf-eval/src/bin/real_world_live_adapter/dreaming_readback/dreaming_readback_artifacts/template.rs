use crate::{LoadedJob, Result, Value, eyre};

pub(in crate::dreaming_readback) fn dreaming_readback_template_artifacts(
	loaded: &LoadedJob,
) -> Result<Vec<Value>> {
	let pointer = match loaded.job.suite.as_str() {
		"memory_summary" => "/corpus/adapter_response/answer/memory_summaries",
		"proactive_brief" => "/corpus/adapter_response/answer/proactive_briefs",
		"scheduled_memory" => "/corpus/adapter_response/answer/scheduled_tasks",
		_ => return Ok(Vec::new()),
	};
	let artifacts =
		loaded.value.pointer(pointer).and_then(Value::as_array).cloned().ok_or_else(|| {
			eyre::eyre!(
				"{} missing service-native readback template at {pointer}.",
				loaded.job.job_id
			)
		})?;

	if artifacts.is_empty() {
		return Err(eyre::eyre!(
			"{} has no service-native readback template artifacts.",
			loaded.job.job_id
		));
	}

	Ok(artifacts)
}
