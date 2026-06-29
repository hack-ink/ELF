mod consolidation;
mod core_memory;
mod docs;
mod errors;
mod events;
mod graph;
mod ingestion_profiles;
mod knowledge;
mod notes;
mod recall;
mod search;
mod sharing;
mod trace;
mod work_journal;

pub(in crate::routes) use self::{
	consolidation::{
		ConsolidationProposalReviewBody, ConsolidationProposalsListQuery,
		ConsolidationRunCreateBody, ConsolidationRunsListQuery, DreamingReviewQueueQuery,
	},
	core_memory::{CoreBlockAttachBody, CoreBlockUpsertBody},
	docs::{DocsExcerptsGetBody, DocsPutBody, DocsSearchL0Body},
	errors::ErrorBody,
	events::EventsIngestRequest,
	graph::{
		AdminGraphPredicateAliasAddBody, AdminGraphPredicatePatchBody,
		AdminGraphPredicatesListQuery, GraphQueryBody, GraphReportBody,
	},
	ingestion_profiles::{
		AdminIngestionProfileCreateBody, AdminIngestionProfileDefaultResponseV2,
		AdminIngestionProfileDefaultSetBody, AdminIngestionProfileGetQuery,
	},
	knowledge::{
		KnowledgePageRebuildBody, KnowledgePageWatchRebuildBody, KnowledgePagesListQuery,
		KnowledgePagesSearchBody,
	},
	notes::{
		AdminNoteCorrectionBody, NotePatchRequest, NotesIngestRequest, NotesListQuery,
		PublishResponseV2,
	},
	recall::RecallDebugPanelBody,
	search::{
		SearchCreateRequest, SearchCreateResponseV2, SearchDetailsBody, SearchDetailsResponseV2,
		SearchIndexResponseV2, SearchSessionGetQuery, SearchTimelineQuery,
		SearchTimelineResponseV2,
	},
	sharing::{
		ShareScopeBody, SpaceGrantItemV2, SpaceGrantUpsertBody, SpaceGrantUpsertResponseV2,
		SpaceGrantsListResponseV2,
	},
	trace::{TraceBundleGetQuery, TraceRecentListQuery},
	work_journal::{WorkJournalEntryCreateBody, WorkJournalSessionReadbackBody},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::routes::{
	AddNoteInput, ConsolidationInputRef, ConsolidationLineage, ConsolidationProposalInput,
	ConsolidationReviewAction, ConsolidationReviewState, DocType, EventMessage, GranteeKind,
	GraphQueryEntityRef, GraphQueryPredicateRef, IngestionProfileSelector, KnowledgePageKind,
	KnowledgeSourceKind, MemoryCorrectionAction, PayloadLevel, QueryPlan, RankingRequestOverride,
	SearchDetailsResult, SearchIndexItem, SearchMode, SearchTimelineGroup, SearchTrajectorySummary,
	TextPositionSelector, TextQuoteSelector, TraceBundleMode, WorkJournalEntryFamily, WritePolicy,
	empty_json_object,
};
