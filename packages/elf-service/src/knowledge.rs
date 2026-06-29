//! Deterministic derived knowledge page rebuild and readback service APIs.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::{self, Map, Number, Value};
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, Error, Result, access,
	consolidation::{
		ConsolidationProposalInput, ConsolidationRunCreateRequest, ConsolidationRunCreateResponse,
	},
	search,
};
use elf_domain::{
	consolidation::{
		ConsolidationApplyIntent, ConsolidationInputRef, ConsolidationLineage, ConsolidationMarker,
		ConsolidationMarkerSeverity, ConsolidationMarkers, ConsolidationProposalDiff,
		ConsolidationSourceKind, ConsolidationSourceSnapshot,
	},
	english_gate,
	knowledge::{
		KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1, KNOWLEDGE_PAGE_REBUILD_SCHEMA_V1,
		KNOWLEDGE_PAGE_SOURCE_COVERAGE_SCHEMA_V1, KNOWLEDGE_PAGE_VERSION_DIFF_SCHEMA_V1,
		KNOWLEDGE_PAGE_WATCH_REBUILD_SCHEMA_V1, KnowledgePageKind, KnowledgeSourceKind,
	},
};
use elf_storage::{
	knowledge::{
		self, KnowledgeDocChunkSource, KnowledgeDocSource, KnowledgeEventSource,
		KnowledgeNoteSource, KnowledgePageLintFindingInsert, KnowledgePageSearchRow,
		KnowledgePageSectionInsert, KnowledgePageSourceRefInsert, KnowledgePageUpsert,
		KnowledgeProposalSource, KnowledgeRelationSource, KnowledgeRelationSourcesFetch,
	},
	models::{KnowledgePage, KnowledgePageSection, KnowledgePageSourceRef},
};

const DEFAULT_LIST_LIMIT: i64 = 50;
const MAX_LIST_LIMIT: i64 = 200;
const SEARCH_SNIPPET_CHARS: usize = 280;
const PREVIOUS_VERSION_DIFF_KEY: &str = "previous_version_diff";

mod api;
pub use api::*;
mod support;
use support::*;
mod types;
use types::*;
mod watch;
use watch::*;
mod sources;
use sources::*;
mod responses;
use responses::*;
mod sections;
use sections::*;
mod persistence;
use persistence::*;
mod lint;
mod read;
mod rebuild;
mod resolve;
mod watch_service;

#[cfg(test)]
#[path = "knowledge/tests.rs"]
mod tests;
