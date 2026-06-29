use super::*;

pub(super) fn validate_graph_report_request(
	req: GraphReportRequest,
) -> Result<PreparedGraphReport> {
	let tenant_id = normalize_required_field(req.tenant_id.as_str(), "tenant_id")?;
	let project_id = normalize_required_field(req.project_id.as_str(), "project_id")?;
	let agent_id = normalize_required_field(req.agent_id.as_str(), "agent_id")?;
	let read_profile = normalize_required_field(req.read_profile.as_str(), "read_profile")?;
	let subject = match req.subject {
		GraphQueryEntityRef::EntityId { entity_id } => GraphQueryEntityRef::EntityId { entity_id },
		GraphQueryEntityRef::Surface { surface } => {
			let surface = normalize_required_field(surface.as_str(), "subject.surface")?;

			GraphQueryEntityRef::Surface { surface }
		},
	};
	let predicate = match req.predicate {
		Some(GraphQueryPredicateRef::PredicateId { predicate_id }) =>
			Some(GraphQueryPredicateRef::PredicateId { predicate_id }),
		Some(GraphQueryPredicateRef::Surface { surface }) => {
			let surface = normalize_required_field(surface.as_str(), "predicate.surface")?;

			Some(GraphQueryPredicateRef::Surface { surface })
		},
		None => None,
	};
	let requested_scopes = normalize_scopes(req.scopes)?;
	let limit = req.limit.unwrap_or(DEFAULT_GRAPH_REPORT_LIMIT);

	if !matches!(limit, 1..=MAX_GRAPH_REPORT_LIMIT) {
		return Err(Error::InvalidRequest {
			message: format!("limit must be between 1 and {MAX_GRAPH_REPORT_LIMIT}."),
		});
	}

	Ok(PreparedGraphReport {
		tenant_id,
		project_id,
		agent_id,
		read_profile,
		subject,
		predicate,
		requested_scopes,
		as_of: req.as_of.unwrap_or_else(OffsetDateTime::now_utc),
		limit: limit as usize,
		explain: req.explain.unwrap_or(false),
	})
}

fn normalize_required_field(value: &str, field: &str) -> Result<String> {
	let trimmed = value.trim();

	if trimmed.is_empty() {
		return Err(Error::InvalidRequest { message: format!("{field} is required.") });
	}

	Ok(trimmed.to_string())
}

fn normalize_scopes(scopes: Option<Vec<String>>) -> Result<Vec<String>> {
	let scopes = scopes.unwrap_or_default();
	let mut seen = BTreeSet::new();
	let mut normalized = Vec::new();

	for scope in scopes {
		let scope = scope.trim().to_string();

		if scope.is_empty() {
			return Err(Error::InvalidRequest {
				message: "scopes entries must be non-empty strings.".to_string(),
			});
		}
		if seen.insert(scope.clone()) {
			normalized.push(scope);
		}
	}

	Ok(normalized)
}
