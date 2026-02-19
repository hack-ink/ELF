CREATE TABLE IF NOT EXISTS graph_entities (
	entity_id uuid PRIMARY KEY,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	canonical text NOT NULL,
	canonical_norm text NOT NULL,
	kind text NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_graph_entities_tenant_project_canonical_norm
	ON graph_entities (tenant_id, project_id, canonical_norm);

