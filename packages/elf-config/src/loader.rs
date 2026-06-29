use std::{fs, path::Path};

use crate::{Config, Error, Result, validation};

/// Loads, deserializes, and validates an ELF TOML configuration file.
pub fn load(path: &Path) -> Result<Config> {
	let raw = fs::read_to_string(path)
		.map_err(|err| Error::ReadConfig { path: path.to_path_buf(), source: err })?;
	let cfg: Config = toml::from_str(&raw)
		.map_err(|err| Error::ParseConfig { path: path.to_path_buf(), source: err })?;

	validation::validate(&cfg)?;

	Ok(cfg)
}
