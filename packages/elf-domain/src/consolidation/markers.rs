use super::*;

/// Confidence or honesty marker severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidationMarkerSeverity {
	/// Low-severity marker.
	Low,
	/// Medium-severity marker.
	Medium,
	/// High-severity marker.
	High,
}

/// One contradiction or staleness marker attached to a proposal.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ConsolidationMarker {
	/// Marker severity.
	pub severity: ConsolidationMarkerSeverity,
	/// Human-readable marker text.
	pub message: String,
	/// Optional source that triggered the marker.
	pub source: Option<ConsolidationInputRef>,
}
impl ConsolidationMarker {
	/// Validates marker content and optional source evidence.
	pub fn validate(&self) -> Result<(), ConsolidationValidationError> {
		if self.message.trim().is_empty() {
			return Err(ConsolidationValidationError::EmptyText { field: "marker.message" });
		}

		if let Some(source) = &self.source {
			source.validate()?;
		}

		Ok(())
	}
}

/// Contradiction and staleness markers attached to a proposal.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct ConsolidationMarkers {
	#[serde(default)]
	/// Contradiction markers that a reviewer must inspect.
	pub contradictions: Vec<ConsolidationMarker>,
	#[serde(default)]
	/// Staleness markers that a reviewer must inspect.
	pub staleness: Vec<ConsolidationMarker>,
}
impl ConsolidationMarkers {
	/// Validates all marker payloads.
	pub fn validate(&self) -> Result<(), ConsolidationValidationError> {
		for marker in self.contradictions.iter().chain(self.staleness.iter()) {
			marker.validate()?;
		}

		Ok(())
	}
}

/// Unsupported-claim marker attached to a proposal for reviewer inspection.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ConsolidationUnsupportedClaimFlag {
	/// Stable claim identifier when the source fixture or worker supplies one.
	pub claim_id: Option<String>,
	/// Human-readable unsupported-claim description.
	pub message: String,
	/// Optional source that demonstrates why the claim is unsupported.
	pub source: Option<ConsolidationInputRef>,
}
impl ConsolidationUnsupportedClaimFlag {
	/// Validates unsupported-claim marker content and optional source evidence.
	pub fn validate(&self) -> Result<(), ConsolidationValidationError> {
		if self.message.trim().is_empty() {
			return Err(ConsolidationValidationError::EmptyText {
				field: "unsupported_claim_flags.message",
			});
		}

		if let Some(claim_id) = &self.claim_id
			&& claim_id.trim().is_empty()
		{
			return Err(ConsolidationValidationError::EmptyText {
				field: "unsupported_claim_flags.claim_id",
			});
		}
		if let Some(source) = &self.source {
			source.validate()?;
		}

		Ok(())
	}
}
