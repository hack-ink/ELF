/// Schema identifier for Work Journal readback.
pub const ELF_WORK_JOURNAL_SCHEMA_V1: &str = "elf.work_journal/v1";

pub(in crate::work_journal) const WORK_JOURNAL_PROMOTION_BOUNDARY_SCHEMA_V1: &str =
	"elf.work_journal.promotion_boundary/v1";
pub(in crate::work_journal) const DEFAULT_SESSION_READBACK_LIMIT: u32 = 20;
pub(in crate::work_journal) const MAX_SESSION_READBACK_LIMIT: u32 = 100;
pub(in crate::work_journal) const MAX_STORAGE_SCAN_ROWS: i64 = 500;
pub(in crate::work_journal) const MAX_BODY_CHARS: usize = 16_384;
pub(in crate::work_journal) const MAX_SIDE_LIST_ITEMS: usize = 64;
