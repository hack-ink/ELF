//! Writegate validation and redaction helpers.

mod policy;
mod secrets;
mod types;
mod validation;

pub use self::{
	policy::apply_write_policy,
	secrets::contains_secrets,
	types::{
		NoteInput, RejectCode, WritePolicy, WritePolicyAudit, WritePolicyError, WritePolicyResult,
		WriteRedaction, WriteRedactionResult, WriteSpan,
	},
	validation::writegate,
};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::english_gate;
use elf_config::Config;

#[cfg(test)] mod tests;
