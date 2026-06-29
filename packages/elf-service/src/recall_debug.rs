//! Cross-layer recall/debug panel readback.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	DocsSearchL0Request, DreamingReviewQueueRequest, ElfService, Error, GraphQueryEntityRef,
	GraphQueryPredicateRef, GraphReportRequest, KnowledgePageSearchItem,
	KnowledgePageSearchRequest, Result, SearchExplainItem, SearchTrace, SearchTrajectoryStage,
	TraceBundleGetRequest,
	access::{self, ORG_PROJECT_ID, SharedSpaceGrantKey},
	search::{self, TraceBundleMode, TraceReplayCandidate},
};
use elf_storage::models::MemoryNote;

mod types;
pub use types::*;
mod helpers;
use helpers::*;
mod sources;
use sources::*;
mod replay;
use replay::*;
mod trace;
use trace::*;
mod layers;

#[cfg(test)]
#[path = "recall_debug/tests.rs"]
mod tests;
