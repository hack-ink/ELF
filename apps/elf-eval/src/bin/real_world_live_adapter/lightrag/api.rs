use std::{future::Future, time::Duration};

use reqwest::{Client, RequestBuilder};
use tokio::time;

use crate::{
	CorpusText, LightragArgs, LightragSource, LoadedJob, Result, eyre,
	lightrag::{corpus, metadata, status},
	serde_json,
};

#[derive(Debug, Eq, PartialEq)]
enum LightragClearStatus {
	Completed,
	Busy,
	Unexpected,
}

pub(super) async fn wait_for_lightrag(args: &LightragArgs, client: &Client) -> Result<()> {
	let mut last_error = String::new();

	for _attempt in 1..=args.startup_attempts {
		match lightrag_get_json(args, client, "/health").await {
			Ok(_) => return Ok(()),
			Err(err) => last_error = err.to_string(),
		}

		time::sleep(Duration::from_secs(args.startup_interval_seconds)).await;
	}

	Err(eyre::eyre!(
		"LightRAG API did not become healthy at {} after {} attempts: {}",
		metadata::lightrag_api_base(args),
		args.startup_attempts,
		last_error
	))
}

pub(super) async fn clear_lightrag_documents(
	args: &LightragArgs,
	client: &Client,
) -> Result<serde_json::Value> {
	clear_lightrag_documents_with(args.clear_attempts, args.index_interval_seconds, || {
		lightrag_delete_json(args, client, "/documents")
	})
	.await
}

pub(super) async fn insert_lightrag_texts(
	args: &LightragArgs,
	client: &Client,
	corpus: &[CorpusText],
	sources: &[LightragSource],
) -> Result<serde_json::Value> {
	let request = serde_json::json!({
		"texts": corpus.iter().map(|item| item.text.as_str()).collect::<Vec<_>>(),
		"file_sources": sources.iter().map(|source| source.file_source.as_str()).collect::<Vec<_>>(),
		"chunking": {
			"strategy": "fixed_token",
			"params": {
				"chunk_token_size": 320,
				"chunk_overlap_token_size": 32
			}
		}
	});

	lightrag_post_json(args, client, "/documents/texts", &request).await
}

pub(super) async fn wait_for_lightrag_index(
	args: &LightragArgs,
	client: &Client,
	insert_response: &serde_json::Value,
	expected_docs: usize,
) -> Result<()> {
	let track_id = insert_response
		.get("track_id")
		.and_then(serde_json::Value::as_str)
		.ok_or_else(|| eyre::eyre!("LightRAG text insert response did not include track_id."))?;
	let mut last_status = serde_json::Value::Null;

	for attempt in 1..=args.index_attempts {
		let status =
			lightrag_get_json(args, client, format!("/documents/track_status/{track_id}")).await?;

		if status::lightrag_index_failed(&status) {
			return Err(eyre::eyre!(
				"LightRAG document indexing failed for track_id {track_id}: {}",
				serde_json::to_string(&status)?
			));
		}
		if status::lightrag_index_processed(&status, expected_docs) {
			return Ok(());
		}

		last_status = status;

		if attempt < args.index_attempts {
			time::sleep(Duration::from_secs(args.index_interval_seconds)).await;
		}
	}

	Err(eyre::eyre!(
		"LightRAG document indexing did not finish for track_id {} after {} attempts: {}",
		track_id,
		args.index_attempts,
		serde_json::to_string(&last_status)?
	))
}

pub(super) async fn query_lightrag_context(
	args: &LightragArgs,
	client: &Client,
	loaded: &LoadedJob,
) -> Result<serde_json::Value> {
	let keywords = corpus::lightrag_keywords(loaded.job.prompt.content.as_str());
	let request = serde_json::json!({
		"query": loaded.job.prompt.content,
		"mode": args.query_mode,
		"only_need_context": true,
		"include_references": true,
		"include_chunk_content": true,
		"enable_rerank": false,
		"top_k": args.top_k,
		"chunk_top_k": args.chunk_top_k,
		"hl_keywords": keywords,
		"ll_keywords": keywords,
		"stream": false
	});

	lightrag_post_json(args, client, "/query", &request).await
}

fn lightrag_clear_status(response: &serde_json::Value) -> LightragClearStatus {
	match response.get("status").and_then(serde_json::Value::as_str) {
		Some("success") => LightragClearStatus::Completed,
		Some("busy") => LightragClearStatus::Busy,
		_ => LightragClearStatus::Unexpected,
	}
}

async fn clear_lightrag_documents_with<F, Fut>(
	clear_attempts: u32,
	interval_seconds: u64,
	mut delete_documents: F,
) -> Result<serde_json::Value>
where
	F: FnMut() -> Fut,
	Fut: Future<Output = Result<serde_json::Value>>,
{
	let mut last_response = serde_json::Value::Null;

	for attempt in 1..=clear_attempts {
		let response = delete_documents().await?;

		match lightrag_clear_status(&response) {
			LightragClearStatus::Completed => return Ok(response),
			LightragClearStatus::Busy => {
				last_response = response;

				if attempt < clear_attempts {
					time::sleep(Duration::from_secs(interval_seconds)).await;
				}
			},
			LightragClearStatus::Unexpected => {
				return Err(eyre::eyre!(
					"LightRAG document clear did not complete successfully: {}",
					serde_json::to_string(&response)?
				));
			},
		}
	}

	Err(eyre::eyre!(
		"LightRAG document clear stayed busy after {} attempts: {}",
		clear_attempts,
		serde_json::to_string(&last_response)?
	))
}
async fn lightrag_get_json(
	args: &LightragArgs,
	client: &Client,
	path: impl AsRef<str>,
) -> Result<serde_json::Value> {
	let url = format!("{}{}", metadata::lightrag_api_base(args), path.as_ref());
	let mut request = client.get(url);

	if let Some(api_key) = args.api_key.as_deref().filter(|key| !key.is_empty()) {
		request = request.bearer_auth(api_key);
	}

	lightrag_send_json(request).await
}

async fn lightrag_post_json(
	args: &LightragArgs,
	client: &Client,
	path: &str,
	body: &serde_json::Value,
) -> Result<serde_json::Value> {
	let url = format!("{}{}", metadata::lightrag_api_base(args), path);
	let mut request = client.post(url).json(body);

	if let Some(api_key) = args.api_key.as_deref().filter(|key| !key.is_empty()) {
		request = request.bearer_auth(api_key);
	}

	lightrag_send_json(request).await
}

async fn lightrag_delete_json(
	args: &LightragArgs,
	client: &Client,
	path: &str,
) -> Result<serde_json::Value> {
	let url = format!("{}{}", metadata::lightrag_api_base(args), path);
	let mut request = client.delete(url);

	if let Some(api_key) = args.api_key.as_deref().filter(|key| !key.is_empty()) {
		request = request.bearer_auth(api_key);
	}

	lightrag_send_json(request).await
}

async fn lightrag_send_json(request: RequestBuilder) -> Result<serde_json::Value> {
	let response = request.send().await?;
	let status = response.status();
	let body = response.text().await?;

	if !status.is_success() {
		return Err(eyre::eyre!("LightRAG API returned HTTP {status}: {body}"));
	}

	serde_json::from_str(&body)
		.map_err(|err| eyre::eyre!("LightRAG API returned invalid JSON: {err}; body={body}"))
}

#[cfg(test)]
mod tests {
	use std::{collections::VecDeque, future};

	use crate::{
		Result, eyre,
		lightrag::api::{self, LightragClearStatus},
		serde_json,
	};

	#[test]
	fn classifies_lightrag_clear_responses() {
		assert_eq!(
			api::lightrag_clear_status(&serde_json::json!({"status": "success"})),
			LightragClearStatus::Completed
		);
		assert_eq!(
			api::lightrag_clear_status(&serde_json::json!({"status": "busy"})),
			LightragClearStatus::Busy
		);
		assert_eq!(
			api::lightrag_clear_status(&serde_json::json!({"status": "failed"})),
			LightragClearStatus::Unexpected
		);
	}

	#[tokio::test]
	async fn retries_busy_clear_until_success() -> Result<()> {
		let mut responses = VecDeque::from([
			serde_json::json!({"status": "busy"}),
			serde_json::json!({"status": "success"}),
		]);
		let response = api::clear_lightrag_documents_with(2, 0, || {
			let next = Ok(responses.pop_front().unwrap_or(serde_json::Value::Null));

			future::ready(next)
		})
		.await?;

		assert_eq!(response.get("status").and_then(serde_json::Value::as_str), Some("success"));
		assert!(responses.is_empty());

		Ok(())
	}

	#[tokio::test]
	async fn reports_busy_clear_attempt_exhaustion() -> Result<()> {
		let mut responses = VecDeque::from([
			serde_json::json!({"status": "busy"}),
			serde_json::json!({"status": "busy"}),
		]);
		let result = api::clear_lightrag_documents_with(2, 0, || {
			let next = Ok(responses.pop_front().unwrap_or(serde_json::Value::Null));

			future::ready(next)
		})
		.await;
		let error = match result {
			Ok(_) => return Err(eyre::eyre!("busy clear unexpectedly completed")),
			Err(error) => error,
		};

		assert!(error.to_string().contains("stayed busy after 2 attempts"));
		assert!(responses.is_empty());

		Ok(())
	}
}
