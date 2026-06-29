use std::env;

use color_eyre::{Result, eyre};
use reqwest::{
	Client, StatusCode,
	header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
};
use serde::Deserialize;

use super::types::{ProjectObservation, RadarMode, RadarProject, ReleaseObservation};

#[derive(Debug, Deserialize)]
struct GithubRepoResponse {
	html_url: String,
	default_branch: Option<String>,
	pushed_at: Option<String>,
	updated_at: Option<String>,
	stargazers_count: Option<u64>,
	open_issues_count: Option<u64>,
	description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseResponse {
	tag_name: String,
	html_url: String,
	published_at: Option<String>,
}

pub(super) fn github_client(token_env: &str) -> Result<Option<Client>> {
	let mut headers = HeaderMap::new();

	headers.insert(USER_AGENT, HeaderValue::from_static("elf-external-memory-pattern-radar"));
	headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));

	if let Ok(token) = env::var(token_env)
		&& !token.trim().is_empty()
	{
		let value = format!("Bearer {}", token.trim()).parse()?;

		headers.insert(AUTHORIZATION, value);
	}

	Ok(Some(Client::builder().default_headers(headers).build()?))
}

pub(super) async fn observe_project(
	project: &RadarProject,
	mode: RadarMode,
	client: Option<&Client>,
	generated_at: &str,
) -> Result<ProjectObservation> {
	match mode {
		RadarMode::Offline => Ok(project
			.last_seen
			.clone()
			.unwrap_or_else(|| fallback_observation(project, generated_at))),
		RadarMode::Live =>
			fetch_project(
				project,
				client.ok_or_else(|| eyre::eyre!("missing GitHub client"))?,
				generated_at,
			)
			.await,
	}
}

fn fallback_observation(project: &RadarProject, generated_at: &str) -> ProjectObservation {
	ProjectObservation {
		observed_at: generated_at.to_string(),
		source_url: project.homepage.clone(),
		default_branch: None,
		pushed_at: None,
		updated_at: None,
		latest_release: None,
		stars: None,
		open_issues: None,
		description: None,
	}
}

async fn fetch_project(
	project: &RadarProject,
	client: &Client,
	generated_at: &str,
) -> Result<ProjectObservation> {
	let repo = fetch_repo(project, client).await?;
	let latest_release = fetch_latest_release(project, client).await?;

	Ok(ProjectObservation {
		observed_at: generated_at.to_string(),
		source_url: repo.html_url,
		default_branch: repo.default_branch,
		pushed_at: repo.pushed_at,
		updated_at: repo.updated_at,
		latest_release,
		stars: repo.stargazers_count,
		open_issues: repo.open_issues_count,
		description: repo.description,
	})
}

async fn fetch_repo(project: &RadarProject, client: &Client) -> Result<GithubRepoResponse> {
	let url = format!("https://api.github.com/repos/{}", project.repo);
	let response = client.get(url).send().await?;

	if !response.status().is_success() {
		return Err(eyre::eyre!(
			"GitHub repo metadata fetch failed for {} with status {}",
			project.repo,
			response.status()
		));
	}

	Ok(response.json().await?)
}

async fn fetch_latest_release(
	project: &RadarProject,
	client: &Client,
) -> Result<Option<ReleaseObservation>> {
	let url = format!("https://api.github.com/repos/{}/releases/latest", project.repo);
	let response = client.get(url).send().await?;

	if response.status() == StatusCode::NOT_FOUND {
		return Ok(None);
	}
	if !response.status().is_success() {
		return Err(eyre::eyre!(
			"GitHub release metadata fetch failed for {} with status {}",
			project.repo,
			response.status()
		));
	}

	let release: GithubReleaseResponse = response.json().await?;

	Ok(Some(ReleaseObservation {
		tag_name: release.tag_name,
		url: release.html_url,
		published_at: release.published_at,
	}))
}
