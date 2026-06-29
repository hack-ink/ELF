use std::time::Duration;

use reqwest::RequestBuilder;
use tokio::time;

use super::{
	super::*,
	corpus::lightrag_keywords,
	metadata::lightrag_api_base,
	status::{lightrag_index_failed, lightrag_index_processed},
};

pub(super) async fn wait_for_lightrag(
	args: &LightragArgs,
	client: &reqwest::Client,
) -> color_eyre::Result<()> {
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
		lightrag_api_base(args),
		args.startup_attempts,
		last_error
	))
}

pub(super) async fn insert_lightrag_texts(
	args: &LightragArgs,
	client: &reqwest::Client,
	corpus: &[CorpusText],
	sources: &[LightragSource],
) -> color_eyre::Result<serde_json::Value> {
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
	client: &reqwest::Client,
	insert_response: &serde_json::Value,
	expected_docs: usize,
) -> color_eyre::Result<()> {
	let track_id = insert_response
		.get("track_id")
		.and_then(serde_json::Value::as_str)
		.ok_or_else(|| eyre::eyre!("LightRAG text insert response did not include track_id."))?;
	let mut last_status = serde_json::Value::Null;

	for _attempt in 1..=args.index_attempts {
		let status =
			lightrag_get_json(args, client, format!("/documents/track_status/{track_id}")).await?;

		if lightrag_index_failed(&status) {
			return Err(eyre::eyre!(
				"LightRAG document indexing failed for track_id {track_id}: {}",
				serde_json::to_string(&status)?
			));
		}
		if lightrag_index_processed(&status, expected_docs) {
			return Ok(());
		}

		last_status = status;

		time::sleep(Duration::from_secs(args.index_interval_seconds)).await;
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
	client: &reqwest::Client,
	loaded: &LoadedJob,
) -> color_eyre::Result<serde_json::Value> {
	let keywords = lightrag_keywords(loaded.job.prompt.content.as_str());
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

async fn lightrag_get_json(
	args: &LightragArgs,
	client: &reqwest::Client,
	path: impl AsRef<str>,
) -> color_eyre::Result<serde_json::Value> {
	let url = format!("{}{}", lightrag_api_base(args), path.as_ref());
	let mut request = client.get(url);

	if let Some(api_key) = args.api_key.as_deref().filter(|key| !key.is_empty()) {
		request = request.bearer_auth(api_key);
	}

	lightrag_send_json(request).await
}

async fn lightrag_post_json(
	args: &LightragArgs,
	client: &reqwest::Client,
	path: &str,
	body: &serde_json::Value,
) -> color_eyre::Result<serde_json::Value> {
	let url = format!("{}{}", lightrag_api_base(args), path);
	let mut request = client.post(url).json(body);

	if let Some(api_key) = args.api_key.as_deref().filter(|key| !key.is_empty()) {
		request = request.bearer_auth(api_key);
	}

	lightrag_send_json(request).await
}

async fn lightrag_send_json(request: RequestBuilder) -> color_eyre::Result<serde_json::Value> {
	let response = request.send().await?;
	let status = response.status();
	let body = response.text().await?;

	if !status.is_success() {
		return Err(eyre::eyre!("LightRAG API returned HTTP {status}: {body}"));
	}

	serde_json::from_str(&body)
		.map_err(|err| eyre::eyre!("LightRAG API returned invalid JSON: {err}; body={body}"))
}
