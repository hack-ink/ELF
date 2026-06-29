#[path = "schemas/admin.rs"] mod admin;
#[path = "schemas/docs.rs"] mod docs;
#[path = "schemas/events.rs"] mod events;
#[path = "schemas/graph.rs"] mod graph;
#[path = "schemas/memory.rs"] mod memory;
#[path = "schemas/notes.rs"] mod notes;
#[path = "schemas/search.rs"] mod search;
#[path = "schemas/sharing.rs"] mod sharing;
#[path = "schemas/work_journal.rs"] mod work_journal;

pub(in crate::app::server) use self::{
	admin::*, docs::*, events::*, graph::*, memory::*, notes::*, search::*, sharing::*,
	work_journal::*,
};
