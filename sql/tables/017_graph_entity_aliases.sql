CREATE TABLE IF NOT EXISTS graph_entity_aliases (
	alias_id uuid PRIMARY KEY,
	entity_id uuid NOT NULL REFERENCES graph_entities(entity_id) ON DELETE CASCADE,
	alias text NOT NULL,
	alias_norm text NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_graph_entity_aliases_entity_alias_norm
	ON graph_entity_aliases (entity_id, alias_norm);
CREATE INDEX IF NOT EXISTS idx_graph_entity_aliases_alias_norm
	ON graph_entity_aliases (alias_norm);

