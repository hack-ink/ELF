pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Failed to read config file at {path:?}.")]
	ReadConfig { path: std::path::PathBuf, source: std::io::Error },
	#[error("Failed to parse config file at {path:?}.")]
	ParseConfig { path: std::path::PathBuf, source: toml::de::Error },
	#[error("{message}")]
	Validation { message: String },
}
