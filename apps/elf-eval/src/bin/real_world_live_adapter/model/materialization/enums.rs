use crate::{Serialize, ValueEnum};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AdapterKind {
	ElfServiceRuntime,
	QmdCliRuntime,
	LightragApiContextExport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MaterializationStatus {
	Pass,
	WrongResult,
	Blocked,
	Incomplete,
	NotEncoded,
}
