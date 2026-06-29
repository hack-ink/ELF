#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::search) enum ExpansionMode {
	Off,
	Always,
	Dynamic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::search) enum RawSearchPath {
	Quick,
	Planned,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::search) enum RetrievalSourceKind {
	Fusion,
	StructuredField,
	Recursive,
}
