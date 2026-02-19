CREATE TABLE IF NOT EXISTS graph_predicates (
	predicate_id uuid PRIMARY KEY,
	scope_key text NOT NULL,
	tenant_id text NULL,
	project_id text NULL,
	canonical text NOT NULL,
	canonical_norm text NOT NULL,
	cardinality text NOT NULL,
	status text NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT graph_predicates_cardinality_check
		CHECK (cardinality IN ('single', 'multi')),
	CONSTRAINT graph_predicates_status_check
		CHECK (status IN ('pending', 'active', 'deprecated'))
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_graph_predicates_scope_canonical_norm
	ON graph_predicates (scope_key, canonical_norm);

CREATE INDEX IF NOT EXISTS idx_graph_predicates_tenant_project_status
	ON graph_predicates (tenant_id, project_id, status);

