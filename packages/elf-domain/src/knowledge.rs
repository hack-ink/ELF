//! Derived knowledge page contract identifiers and storage enums.

use serde::{Deserialize, Serialize};

/// Current derived knowledge page contract schema identifier.
pub const KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1: &str = "elf.knowledge_page/v1";
/// Current deterministic rebuild metadata schema identifier.
pub const KNOWLEDGE_PAGE_REBUILD_SCHEMA_V1: &str = "elf.knowledge_page.rebuild/v1";
/// Current source coverage metadata schema identifier.
pub const KNOWLEDGE_PAGE_SOURCE_COVERAGE_SCHEMA_V1: &str = "elf.knowledge_page.source_coverage/v1";

/// Derived knowledge page category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgePageKind {
	/// Project overview page.
	Project,
	/// Entity dossier page.
	Entity,
	/// Concept page.
	Concept,
	/// Issue timeline or issue dossier page.
	Issue,
	/// Decision page.
	Decision,
}
impl KnowledgePageKind {
	/// Returns the canonical storage string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Project => "project",
			Self::Entity => "entity",
			Self::Concept => "concept",
			Self::Issue => "issue",
			Self::Decision => "decision",
		}
	}

	/// Parses a canonical storage string.
	pub fn parse(raw: &str) -> Option<Self> {
		match raw {
			"project" => Some(Self::Project),
			"entity" => Some(Self::Entity),
			"concept" => Some(Self::Concept),
			"issue" => Some(Self::Issue),
			"decision" => Some(Self::Decision),
			_ => None,
		}
	}
}

/// Authoritative source kind used by a derived page citation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeSourceKind {
	/// Memory note source.
	Note,
	/// Event source reserved for future durable event rows.
	Event,
	/// Graph relation fact source.
	Relation,
	/// Reviewed consolidation proposal source.
	Proposal,
}
impl KnowledgeSourceKind {
	/// Returns the canonical storage string.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Note => "note",
			Self::Event => "event",
			Self::Relation => "relation",
			Self::Proposal => "proposal",
		}
	}

	/// Parses a canonical storage string.
	pub fn parse(raw: &str) -> Option<Self> {
		match raw {
			"note" => Some(Self::Note),
			"event" => Some(Self::Event),
			"relation" => Some(Self::Relation),
			"proposal" => Some(Self::Proposal),
			_ => None,
		}
	}
}
