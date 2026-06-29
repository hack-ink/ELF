use super::{
	formatting::status_str,
	recovery::{
		REQUIRED_AUTHORITY_PLANES, recovery_dead_letter_succeeded, recovery_measurement_met,
		recovery_migration_repair_succeeded, recovery_outbox_replay_succeeded,
		recovery_qdrant_rebuild_succeeded,
	},
	*,
};

#[path = "validation/adapter.rs"] mod adapter;
#[path = "validation/basics.rs"] mod basics;
#[path = "validation/common.rs"] mod common;
#[path = "validation/consolidation.rs"] mod consolidation;
#[path = "validation/expectations.rs"] mod expectations;
#[path = "validation/job_rules.rs"] mod job_rules;
#[path = "validation/memory_summary.rs"] mod memory_summary;
#[path = "validation/page.rs"] mod page;
#[path = "validation/proactive.rs"] mod proactive;
#[path = "validation/recovery_artifact.rs"] mod recovery_artifact;
#[path = "validation/scheduled.rs"] mod scheduled;
#[path = "validation/trace.rs"] mod trace;
#[path = "validation/work_journal.rs"] mod work_journal;

use self::{
	adapter::*, basics::*, common::*, consolidation::*, expectations::*, job_rules::*,
	memory_summary::*, page::*, proactive::*, recovery_artifact::*, scheduled::*, trace::*,
	work_journal::*,
};

pub(super) fn validate_job(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.schema != JOB_SCHEMA {
		return Err(eyre::eyre!(
			"{} has schema {}, expected {JOB_SCHEMA}.",
			path.display(),
			job.schema
		));
	}

	validate_job_identity(job, path)?;

	if !SUITES.contains(&job.suite.as_str()) {
		return Err(eyre::eyre!("{} uses unknown suite {}.", path.display(), job.suite));
	}

	validate_corpus_items(job, path)?;
	validate_timeline(job, path)?;
	validate_prompt(job, path)?;
	validate_expected_answer(job, path)?;
	validate_required_evidence(job, path)?;
	validate_consolidation_fixture(job, path)?;
	validate_adapter_response(job, path)?;
	validate_scoring_rubric(job, path)?;
	validate_allowed_uncertainty(job, path)?;
	validate_operator_debug(job, path)?;
	validate_job_encoding(job, path)?;
	validate_memory_evolution(job, path)?;
	validate_memory_summary_expectation(job, path)?;
	validate_proactive_brief_expectation(job, path)?;
	validate_scheduled_memory_expectation(job, path)?;
	validate_work_continuity_expectation(job, path)?;
	validate_trace_explainability(job, path)?;

	Ok(())
}
