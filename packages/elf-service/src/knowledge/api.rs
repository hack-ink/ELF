mod readback;
mod requests;
mod search;
mod watch;

pub use self::{
	readback::{
		KnowledgePageLintFindingResponse, KnowledgePageLintResponse, KnowledgePageRebuildResponse,
		KnowledgePageResponse, KnowledgePageSectionResponse, KnowledgePageSectionSourceBacklink,
		KnowledgePageSourceRefResponse, KnowledgePageSummary, KnowledgePagesListResponse,
	},
	requests::{
		KnowledgePageChangedSource, KnowledgePageGetRequest, KnowledgePageLintRequest,
		KnowledgePageRebuildRequest, KnowledgePageSearchRequest, KnowledgePageWatchRebuildRequest,
		KnowledgePagesListRequest,
	},
	search::{KnowledgePageLintSummary, KnowledgePageSearchItem, KnowledgePageSearchResponse},
	watch::{
		KnowledgeDeltaMemoryCandidate, KnowledgePageProposalRunSummary, KnowledgePageRebuildOutput,
		KnowledgePageSectionRebuildState, KnowledgePageWatchRebuildItem,
		KnowledgePageWatchRebuildResponse, KnowledgePageWatchRebuildSummary,
	},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::knowledge::{
	default_generate_memory_candidates, empty_object, previous_version_diff_from_metadata,
	repair_guidance_for_finding_type,
};
use elf_domain::{
	consolidation::{ConsolidationInputRef, ConsolidationProposalDiff},
	knowledge::{KnowledgePageKind, KnowledgeSourceKind},
};
use elf_storage::models::{
	KnowledgePage, KnowledgePageLintFinding, KnowledgePageSection, KnowledgePageSourceRef,
};
