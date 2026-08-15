use color_eyre::eyre;

use crate::{Command, Path, QmdArgs, Result, fs};

pub(super) fn ensure_qmd_checkout(args: &QmdArgs, log_path: &Path) -> Result<()> {
	if !args.qmd_dir.exists() {
		if let Some(parent) = args.qmd_dir.parent() {
			fs::create_dir_all(parent)?;
		}

		crate::run_logged_command(
			"qmd clone",
			Command::new("git").arg("clone").arg(&args.qmd_repo_url).arg(&args.qmd_dir),
			log_path,
		)?;
		crate::run_logged_command(
			"qmd checkout pin",
			Command::new("git")
				.arg("-C")
				.arg(&args.qmd_dir)
				.arg("checkout")
				.arg("--detach")
				.arg(&args.qmd_revision),
			log_path,
		)?;
	}

	let observed = crate::run_logged_command(
		"qmd verify pin",
		Command::new("git").arg("-C").arg(&args.qmd_dir).arg("rev-parse").arg("HEAD"),
		log_path,
	)?;

	if observed.trim() != args.qmd_revision {
		return Err(eyre::eyre!(
			"qmd checkout is {}, expected {}.",
			observed.trim(),
			args.qmd_revision
		));
	}
	if args.qmd_dir.join("node_modules/.bin/tsx").is_file() {
		return Ok(());
	}

	crate::run_logged_shell(
		"qmd install",
		&args.qmd_dir,
		"pnpm install --frozen-lockfile && pnpm run build --if-present",
		log_path,
	)
}
