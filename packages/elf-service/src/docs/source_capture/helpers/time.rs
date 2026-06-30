use crate::docs::{Error, OffsetDateTime, Result, Rfc3339};

pub(in crate::docs::source_capture) fn format_timestamp(ts: OffsetDateTime) -> Result<String> {
	ts.format(&Rfc3339).map_err(|err| Error::InvalidRequest {
		message: format!("failed to format RFC3339 timestamp: {err}"),
	})
}
