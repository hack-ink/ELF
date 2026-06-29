use super::super::*;

pub(in crate::docs) fn build_doc_search_filter(
	tenant_id: &str,
	project_id: &str,
	caller_agent_id: &str,
	allowed_scopes: &[String],
	filters: &DocsSearchL0Filters,
) -> Filter {
	let private_scope = "agent_private".to_string();
	let non_private_scopes: Vec<String> =
		allowed_scopes.iter().filter(|scope| *scope != "agent_private").cloned().collect();
	let mut scope_should_conditions = Vec::new();

	if allowed_scopes.iter().any(|scope| scope == "agent_private") {
		let private_filter = Filter::all([
			Condition::matches("scope", private_scope),
			Condition::matches("agent_id", caller_agent_id.to_string()),
		]);

		scope_should_conditions.push(Condition::from(private_filter));
	}
	if !non_private_scopes.is_empty() {
		scope_should_conditions.push(Condition::matches("scope", non_private_scopes));
	}

	let scope_min_should = if scope_should_conditions.is_empty() {
		None
	} else {
		Some(MinShould { min_count: 1, conditions: scope_should_conditions })
	};
	let mut project_or_org_branches = vec![Condition::from(Filter {
		must: vec![Condition::matches("project_id", project_id.to_string())],
		should: Vec::new(),
		must_not: Vec::new(),
		min_should: scope_min_should,
	})];

	if allowed_scopes.iter().any(|scope| scope == "org_shared") {
		let org_filter = Filter::all([
			Condition::matches("project_id", ORG_PROJECT_ID.to_string()),
			Condition::matches("scope", "org_shared".to_string()),
		]);

		project_or_org_branches.push(Condition::from(org_filter));
	}

	Filter {
		must: {
			let mut must = vec![
				Condition::matches("tenant_id", tenant_id.to_string()),
				Condition::matches("status", filters.status.clone()),
			];

			if let Some(scope) = filters.scope.as_ref() {
				must.push(Condition::matches("scope", scope.to_string()));
			}
			if let Some(doc_type) = filters.doc_type.as_ref() {
				must.push(Condition::matches("doc_type", doc_type.as_str().to_string()));
			}
			if let Some(domain) = filters.domain.as_ref() {
				must.push(Condition::matches("domain", domain.to_string()));
			}
			if let Some(repo) = filters.repo.as_ref() {
				must.push(Condition::matches("repo", repo.to_string()));
			}
			if let Some(agent_id) = filters.agent_id.as_ref() {
				must.push(Condition::matches("agent_id", agent_id.to_string()));
			}
			if let Some(thread_id) = filters.thread_id.as_ref() {
				must.push(Condition::matches("thread_id", thread_id.to_string()));
			}
			if let Some(datetime_filter) = datetime_filter_range(
				filters.updated_after.as_ref(),
				filters.updated_before.as_ref(),
			) {
				must.push(datetime_filter);
			}
			if let Some(datetime_filter) =
				doc_ts_filter_range(filters.ts_gte.as_ref(), filters.ts_lte.as_ref())
			{
				must.push(datetime_filter);
			}

			must
		},
		should: Vec::new(),
		must_not: Vec::new(),
		min_should: Some(MinShould { min_count: 1, conditions: project_or_org_branches }),
	}
}

pub(in crate::docs) fn datetime_filter_range(
	updated_after: Option<&OffsetDateTime>,
	updated_before: Option<&OffsetDateTime>,
) -> Option<Condition> {
	let gt = updated_after.map(|updated_after| Timestamp {
		seconds: updated_after.unix_timestamp(),
		nanos: updated_after.nanosecond() as i32,
	});
	let lt = updated_before.map(|updated_before| Timestamp {
		seconds: updated_before.unix_timestamp(),
		nanos: updated_before.nanosecond() as i32,
	});

	if gt.is_none() && lt.is_none() {
		return None;
	}

	Some(Condition::datetime_range("updated_at", DatetimeRange { lt, gt, gte: None, lte: None }))
}

pub(in crate::docs) fn doc_ts_filter_range(
	ts_gte: Option<&OffsetDateTime>,
	ts_lte: Option<&OffsetDateTime>,
) -> Option<Condition> {
	let gte = ts_gte.map(|ts_gte| Timestamp {
		seconds: ts_gte.unix_timestamp(),
		nanos: ts_gte.nanosecond() as i32,
	});
	let lte = ts_lte.map(|ts_lte| Timestamp {
		seconds: ts_lte.unix_timestamp(),
		nanos: ts_lte.nanosecond() as i32,
	});

	if gte.is_none() && lte.is_none() {
		return None;
	}

	Some(Condition::datetime_range("doc_ts", DatetimeRange { lt: None, gt: None, gte, lte }))
}
