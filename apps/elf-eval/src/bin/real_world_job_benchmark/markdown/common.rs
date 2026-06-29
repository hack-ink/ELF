use super::*;

pub(super) fn optional_f64(value: Option<f64>, suffix: &str) -> String {
	value.map(|value| format!("{value:.3}{suffix}")).unwrap_or_else(|| "-".to_string())
}

pub(super) fn bool_display(value: bool) -> &'static str {
	if value { "true" } else { "false" }
}

pub(super) fn cost_display(cost: Option<&CostReport>) -> String {
	let Some(cost) = cost else {
		return "-".to_string();
	};

	match (cost.amount, cost.currency.as_deref()) {
		(Some(amount), Some(currency)) => format!("{amount:.3} {currency}"),
		(Some(amount), None) => format!("{amount:.3}"),
		(None, _) => "-".to_string(),
	}
}

pub(super) fn md_inline(value: &str) -> String {
	value.replace('`', "'").replace('\n', " ")
}

pub(super) fn md_cell(value: &str) -> String {
	md_inline(value).replace('|', "\\|")
}

pub(super) fn md_url(value: &str) -> String {
	value.replace(')', "%29").replace(' ', "%20")
}

pub(super) fn md_list(values: &[String]) -> String {
	if values.is_empty() {
		return "-".to_string();
	}

	md_cell(values.join("; ").as_str())
}
