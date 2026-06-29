use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use super::{
	AddNoteInput, ConsolidationInputRef, ConsolidationLineage, ConsolidationProposalInput,
	ConsolidationReviewAction, ConsolidationReviewState, DocType, EventMessage, GranteeKind,
	GraphQueryEntityRef, GraphQueryPredicateRef, IngestionProfileSelector, KnowledgePageKind,
	KnowledgeSourceKind, MemoryCorrectionAction, PayloadLevel, QueryPlan, RankingRequestOverride,
	SearchDetailsResult, SearchIndexItem, SearchMode, SearchTimelineGroup, SearchTrajectorySummary,
	TextPositionSelector, TextQuoteSelector, TraceBundleMode, WorkJournalEntryFamily, WritePolicy,
	empty_json_object,
};

#[path = "types/consolidation.rs"] mod consolidation;
#[path = "types/core_memory.rs"] mod core_memory;
#[path = "types/docs.rs"] mod docs;
#[path = "types/errors.rs"] mod errors;
#[path = "types/events.rs"] mod events;
#[path = "types/graph.rs"] mod graph;
#[path = "types/ingestion_profiles.rs"] mod ingestion_profiles;
#[path = "types/knowledge.rs"] mod knowledge;
#[path = "types/notes.rs"] mod notes;
#[path = "types/recall.rs"] mod recall;
#[path = "types/search.rs"] mod search;
#[path = "types/sharing.rs"] mod sharing;
#[path = "types/trace.rs"] mod trace;
#[path = "types/work_journal.rs"] mod work_journal;

pub(in crate::routes) use self::{
	consolidation::*, core_memory::*, docs::*, errors::*, events::*, graph::*,
	ingestion_profiles::*, knowledge::*, notes::*, recall::*, search::*, sharing::*, trace::*,
	work_journal::*,
};
