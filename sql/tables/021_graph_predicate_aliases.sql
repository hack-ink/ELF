CREATE TABLE IF NOT EXISTS graph_predicate_aliases (
	alias_id uuid PRIMARY KEY,
	predicate_id uuid NOT NULL REFERENCES graph_predicates(predicate_id) ON DELETE CASCADE,
	scope_key text NOT NULL,
	alias text NOT NULL,
	alias_norm text NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_graph_predicate_aliases_scope_alias_norm
	ON graph_predicate_aliases (scope_key, alias_norm);

CREATE INDEX IF NOT EXISTS idx_graph_predicate_aliases_predicate
	ON graph_predicate_aliases (predicate_id);

CREATE INDEX IF NOT EXISTS idx_graph_predicate_aliases_alias_norm
	ON graph_predicate_aliases (alias_norm);

