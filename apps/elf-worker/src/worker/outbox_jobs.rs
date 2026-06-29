use crate::worker::{
	self, CLAIM_LEASE_SECONDS, CONSOLIDATION_JOB_LEASE_SECONDS, Db, Error, OffsetDateTime, Result,
	TRACE_OUTBOX_LEASE_SECONDS, ToString, Uuid, WorkerState, consolidation, doc_outbox, outbox,
};

pub(super) async fn process_indexing_outbox_once(state: &WorkerState) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	let job = outbox::claim_next_indexing_outbox_job(&state.db, now, CLAIM_LEASE_SECONDS).await?;
	let Some(job) = job else { return Ok(()) };
	let result = match job.op.as_str() {
		"UPSERT" => worker::handle_upsert(state, &job).await,
		"DELETE" => worker::handle_delete(state, &job).await,
		other => Err(Error::Validation(format!("Unsupported outbox op: {other}."))),
	};

	match result {
		Ok(()) => {
			outbox::mark_indexing_outbox_done(&state.db, job.outbox_id, OffsetDateTime::now_utc())
				.await?;
		},
		Err(err) => {
			tracing::error!(
				error = %err,
				outbox_id = %job.outbox_id,
				note_id = %job.note_id,
				"Outbox job failed."
			);

			mark_failed(&state.db, job.outbox_id, job.attempts, &err).await?;
		},
	}

	Ok(())
}

pub(super) async fn process_doc_indexing_outbox_once(state: &WorkerState) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	let job =
		doc_outbox::claim_next_doc_indexing_outbox_job(&state.db, now, CLAIM_LEASE_SECONDS).await?;
	let Some(job) = job else { return Ok(()) };
	let result = match job.op.as_str() {
		"UPSERT" => worker::handle_doc_upsert(state, &job).await,
		"DELETE" => worker::handle_doc_delete(state, &job).await,
		other => Err(Error::Validation(format!("Unsupported doc outbox op: {other}."))),
	};

	match result {
		Ok(()) => {
			doc_outbox::mark_doc_indexing_outbox_done(
				&state.db,
				job.outbox_id,
				OffsetDateTime::now_utc(),
			)
			.await?;
		},
		Err(err) => {
			tracing::error!(
				error = %err,
				outbox_id = %job.outbox_id,
				doc_id = %job.doc_id,
				chunk_id = %job.chunk_id,
				"Doc outbox job failed."
			);

			mark_doc_failed(&state.db, job.outbox_id, job.attempts, &err).await?;
		},
	}

	Ok(())
}

pub(super) async fn process_trace_outbox_once(state: &WorkerState) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	let job =
		outbox::claim_next_trace_outbox_job(&state.db, now, TRACE_OUTBOX_LEASE_SECONDS).await?;
	let Some(job) = job else { return Ok(()) };
	let result = worker::handle_trace_job(&state.db, &job).await;

	match result {
		Ok(()) => {
			outbox::mark_trace_outbox_done(&state.db, job.outbox_id, OffsetDateTime::now_utc())
				.await?;
		},
		Err(err) => {
			tracing::error!(
				error = %err,
				outbox_id = %job.outbox_id,
				trace_id = %job.trace_id,
				"Search trace outbox job failed."
			);

			mark_trace_failed(&state.db, job.outbox_id, job.attempts, &err).await?;
		},
	}

	Ok(())
}

pub(super) async fn process_consolidation_run_job_once(state: &WorkerState) -> Result<()> {
	let now = OffsetDateTime::now_utc();
	let job = consolidation::claim_next_consolidation_run_job(
		&state.db,
		now,
		CONSOLIDATION_JOB_LEASE_SECONDS,
	)
	.await?;
	let Some(job) = job else { return Ok(()) };
	let result = worker::handle_consolidation_job(&state.db, &job).await;

	match result {
		Ok(()) => {},
		Err(err) => {
			tracing::error!(
				error = %err,
				job_id = %job.job_id,
				run_id = %job.run_id,
				"Consolidation run job failed."
			);

			mark_consolidation_failed(&state.db, job.job_id, job.attempts, &err).await?;
		},
	}

	Ok(())
}

pub(super) async fn mark_failed(
	db: &Db,
	outbox_id: Uuid,
	attempts: i32,
	err: &Error,
) -> Result<()> {
	let next_attempts = attempts.saturating_add(1);
	let backoff = worker::backoff_for_attempt(next_attempts);
	let now = OffsetDateTime::now_utc();
	let available_at = now + backoff;
	let error_text = worker::sanitize_outbox_error(&err.to_string());

	outbox::mark_indexing_outbox_failed(
		db,
		outbox_id,
		next_attempts,
		error_text.as_str(),
		available_at,
		now,
	)
	.await?;

	Ok(())
}

pub(super) async fn mark_doc_failed(
	db: &Db,
	outbox_id: Uuid,
	attempts: i32,
	err: &Error,
) -> Result<()> {
	let next_attempts = attempts.saturating_add(1);
	let backoff = worker::backoff_for_attempt(next_attempts);
	let now = OffsetDateTime::now_utc();
	let available_at = now + backoff;
	let error_text = worker::sanitize_outbox_error(&err.to_string());

	doc_outbox::mark_doc_indexing_outbox_failed(
		db,
		outbox_id,
		next_attempts,
		error_text.as_str(),
		available_at,
		now,
	)
	.await?;

	Ok(())
}

pub(super) async fn mark_trace_failed(
	db: &Db,
	outbox_id: Uuid,
	attempts: i32,
	err: &Error,
) -> Result<()> {
	let next_attempts = attempts.saturating_add(1);
	let backoff = worker::backoff_for_attempt(next_attempts);
	let now = OffsetDateTime::now_utc();
	let available_at = now + backoff;
	let error_text = worker::sanitize_outbox_error(&err.to_string());

	outbox::mark_trace_outbox_failed(
		db,
		outbox_id,
		next_attempts,
		error_text.as_str(),
		available_at,
		now,
	)
	.await?;

	Ok(())
}

pub(super) async fn mark_consolidation_failed(
	db: &Db,
	job_id: Uuid,
	attempts: i32,
	err: &Error,
) -> Result<()> {
	let next_attempts = attempts.saturating_add(1);
	let backoff = worker::backoff_for_attempt(next_attempts);
	let now = OffsetDateTime::now_utc();
	let available_at = now + backoff;
	let error_text = worker::sanitize_outbox_error(&err.to_string());

	consolidation::mark_consolidation_run_job_failed(
		db,
		job_id,
		next_attempts,
		error_text.as_str(),
		available_at,
		now,
	)
	.await?;

	Ok(())
}
