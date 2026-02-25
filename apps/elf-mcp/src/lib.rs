pub mod server;

use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;
use color_eyre::{Result, eyre};

use elf_config::{McpContext, Security};

#[derive(Debug, Parser)]
#[command(
	version = elf_cli::VERSION,
	rename_all = "kebab",
	styles = elf_cli::styles(),
)]
pub struct Args {
	#[arg(long, short = 'c', value_name = "FILE")]
	pub config: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpAuthState {
	Off,
	StaticKeys { bearer_token: String },
}

pub async fn run(args: Args) -> Result<()> {
	let config = elf_config::load(&args.config)?;
	let mcp =
		config.mcp.as_ref().ok_or_else(|| eyre::eyre!("mcp section is required for elf-mcp."))?;
	let auth_state = build_auth_state(&config.security, &config.service.mcp_bind, mcp)?;

	server::serve_mcp(&config.service.mcp_bind, &config.service.http_bind, auth_state, mcp).await
}

fn build_auth_state(security: &Security, mcp_bind: &str, mcp: &McpContext) -> Result<McpAuthState> {
	match security.auth_mode.trim() {
		"off" => {
			enforce_loopback_for_off_mode(mcp_bind)?;

			Ok(McpAuthState::Off)
		},
		"static_keys" => select_static_key(security, mcp),
		other => Err(eyre::eyre!(
			"security.auth_mode must be one of off or static_keys for elf-mcp, got {other}."
		)),
	}
}

fn enforce_loopback_for_off_mode(mcp_bind: &str) -> Result<()> {
	let bind_addr: SocketAddr = mcp_bind.parse().map_err(|err| {
		eyre::eyre!(
			"service.mcp_bind must be a valid socket address when security.auth_mode=off: {err}"
		)
	})?;

	if !bind_addr.ip().is_loopback() {
		return Err(eyre::eyre!(
			"service.mcp_bind must be a loopback address when security.auth_mode=off."
		));
	}

	Ok(())
}

fn select_static_key(security: &Security, mcp: &McpContext) -> Result<McpAuthState> {
	let mut matches = security.auth_keys.iter().filter(|key| {
		key.tenant_id == mcp.tenant_id
			&& key.project_id == mcp.project_id
			&& key.agent_id.as_deref() == Some(mcp.agent_id.as_str())
			&& key.read_profile == mcp.read_profile
	});
	let first = matches.next();
	let has_multiple = matches.next().is_some();

	match (first, has_multiple) {
		(Some(key), false) => Ok(McpAuthState::StaticKeys { bearer_token: key.token.clone() }),
		(None, _) => Err(eyre::eyre!(
			"security.auth_mode=static_keys requires exactly one matching entry in security.auth_keys for mcp context (tenant_id, project_id, agent_id, read_profile). Found zero."
		)),
		(Some(_), true) => Err(eyre::eyre!(
			"security.auth_mode=static_keys requires exactly one matching entry in security.auth_keys for mcp context (tenant_id, project_id, agent_id, read_profile). Found multiple."
		)),
	}
}

#[cfg(test)]
mod tests {
	use crate::{McpAuthState, build_auth_state};
	use elf_config::{McpContext, Security, SecurityAuthKey, SecurityAuthRole};

	fn sample_security(auth_mode: &str, auth_keys: Vec<SecurityAuthKey>) -> Security {
		Security {
			bind_localhost_only: true,
			reject_non_english: true,
			redact_secrets_on_write: true,
			evidence_min_quotes: 1,
			evidence_max_quotes: 5,
			evidence_max_quote_chars: 400,
			auth_mode: auth_mode.to_string(),
			auth_keys,
		}
	}

	fn sample_mcp() -> McpContext {
		McpContext {
			tenant_id: "tenant-a".to_string(),
			project_id: "project-a".to_string(),
			agent_id: "agent-a".to_string(),
			read_profile: "private_plus_project".to_string(),
		}
	}

	fn sample_key(token_id: &str, token: &str) -> SecurityAuthKey {
		SecurityAuthKey {
			token_id: token_id.to_string(),
			token: token.to_string(),
			tenant_id: "tenant-a".to_string(),
			project_id: "project-a".to_string(),
			agent_id: Some("agent-a".to_string()),
			read_profile: "private_plus_project".to_string(),
			role: SecurityAuthRole::User,
		}
	}

	#[test]
	fn off_mode_requires_loopback_mcp_bind() {
		let security = sample_security("off", vec![]);
		let mcp = sample_mcp();
		let err = build_auth_state(&security, "0.0.0.0:9090", &mcp).expect_err("expected error");

		assert!(err.to_string().contains("security.auth_mode=off"), "unexpected error: {err}");
	}

	#[test]
	fn static_keys_mode_selects_single_matching_key() {
		let security = sample_security("static_keys", vec![sample_key("key-1", "token-1")]);
		let mcp = sample_mcp();
		let auth_state = build_auth_state(&security, "127.0.0.1:9090", &mcp).expect("auth state");

		assert_eq!(auth_state, McpAuthState::StaticKeys { bearer_token: "token-1".to_string() });
	}

	#[test]
	fn static_keys_mode_rejects_multiple_matching_keys() {
		let security = sample_security(
			"static_keys",
			vec![sample_key("key-1", "token-1"), sample_key("key-2", "token-2")],
		);
		let mcp = sample_mcp();
		let err = build_auth_state(&security, "127.0.0.1:9090", &mcp).expect_err("expected error");

		assert!(err.to_string().contains("Found multiple"), "unexpected error: {err}");
	}
}
