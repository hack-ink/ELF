//! Writegate validation and redaction helpers.

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::english_gate;
use elf_config::Config;

mod policy;
mod secrets;
mod types;
mod validation;

pub use self::{
	policy::apply_write_policy, secrets::contains_secrets, types::*, validation::writegate,
};

#[cfg(test)] mod tests;
