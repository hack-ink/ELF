/// Result alias for ELF configuration loading and validation.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors returned while reading, parsing, or validating an ELF config file.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// Reading the config file from disk failed.
	#[error("Failed to read config file at {path:?}.")]
	ReadConfig {
		/// Path of the config file that failed to load.
		path: std::path::PathBuf,
		/// Underlying filesystem error.
		source: std::io::Error,
	},
	/// Parsing the TOML config into the typed schema failed.
	#[error("Failed to parse config file at {path:?}.")]
	ParseConfig {
		/// Path of the config file that failed to parse.
		path: std::path::PathBuf,
		/// Underlying TOML decode error.
		source: toml::de::Error,
	},
	/// A semantic validation rule rejected the config contents.
	#[error("{message}")]
	Validation {
		/// Human-readable validation failure message.
		message: String,
	},
}
