use super::*;

pub(super) fn push_if_selected(
	out: &mut Vec<String>,
	evidence_id: &str,
	selected: &BTreeSet<&str>,
) {
	if selected.contains(evidence_id) {
		push_unique(out, evidence_id.to_string());
	}
}
