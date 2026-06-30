use crate::scoring::{FailureCounts, RealWorldJob};

pub(in crate::scoring) fn operator_debug_failure_counts(job: &RealWorldJob) -> FailureCounts {
	let Some(debug) = &job.operator_debug else {
		return FailureCounts {
			operator_debug_missing: usize::from(job.suite == "operator_debugging_ux"),
			..FailureCounts::default()
		};
	};

	FailureCounts {
		operator_debug_raw_sql: usize::from(debug.raw_sql_needed),
		operator_debug_trace_gaps: usize::from(debug.trace_completeness != "complete"),
		operator_debug_repair_unclear: usize::from(debug.repair_action_clarity != "clear"),
		..FailureCounts::default()
	}
}
