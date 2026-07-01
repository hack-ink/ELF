use color_eyre::{Result, eyre};
use serde_json::Value;

pub(super) fn assert_product_queue_items_reference_queue(
	products: &[Value],
	queue: &[Value],
) -> Result<()> {
	let queue_keys = queue
		.iter()
		.filter_map(|item| item.pointer("/key").and_then(Value::as_str))
		.collect::<Vec<_>>();

	for product in products {
		let product_name = product
			.pointer("/product")
			.and_then(Value::as_str)
			.ok_or_else(|| eyre::eyre!("product row is missing product name"))?;
		let queue_item = product
			.pointer("/p4_queue_item")
			.and_then(Value::as_str)
			.ok_or_else(|| eyre::eyre!("product {product_name} is missing p4_queue_item"))?;

		assert!(
			queue_keys.contains(&queue_item),
			"product {product_name} references missing P4 queue item {queue_item}"
		);
	}

	Ok(())
}

pub(super) fn find_matrix_row<'a>(
	rows: &'a [Value],
	adapter: &str,
	dimension: &str,
) -> Result<&'a Value> {
	rows.iter()
		.find(|row| {
			row.pointer("/adapter").and_then(Value::as_str) == Some(adapter)
				&& row.pointer("/dimension").and_then(Value::as_str) == Some(dimension)
		})
		.ok_or_else(|| eyre::eyre!("missing matrix row for {adapter} {dimension}"))
}
