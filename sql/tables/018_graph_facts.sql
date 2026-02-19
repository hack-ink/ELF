CREATE TABLE IF NOT EXISTS graph_facts (
	fact_id uuid PRIMARY KEY,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	agent_id text NOT NULL,
	scope text NOT NULL,
	subject_entity_id uuid NOT NULL REFERENCES graph_entities(entity_id),
	predicate text NOT NULL,
	predicate_id uuid NULL REFERENCES graph_predicates(predicate_id),
	object_entity_id uuid NULL REFERENCES graph_entities(entity_id),
	object_value text NULL,
	valid_from timestamptz NOT NULL,
	valid_to timestamptz NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	updated_at timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT graph_facts_object_exactly_one_source
		CHECK ((object_entity_id IS NULL AND object_value IS NOT NULL)
			OR (object_entity_id IS NOT NULL AND object_value IS NULL)),
	CONSTRAINT graph_facts_valid_window
		CHECK (valid_to IS NULL OR valid_to > valid_from)
);

ALTER TABLE graph_facts ADD COLUMN IF NOT EXISTS predicate_id uuid NULL;

ALTER TABLE graph_facts DROP CONSTRAINT IF EXISTS graph_facts_predicate_id_fkey;
ALTER TABLE graph_facts
	ADD CONSTRAINT graph_facts_predicate_id_fkey
	FOREIGN KEY (predicate_id) REFERENCES graph_predicates(predicate_id);

DROP INDEX IF EXISTS idx_graph_facts_tenant_project_subject_predicate;
DROP INDEX IF EXISTS uq_graph_facts_active_entity_object;
DROP INDEX IF EXISTS uq_graph_facts_active_entity_value;

CREATE INDEX IF NOT EXISTS idx_graph_facts_tenant_project_subject_predicate
	ON graph_facts (tenant_id, project_id, subject_entity_id, predicate_id);
CREATE INDEX IF NOT EXISTS idx_graph_facts_tenant_project_valid_to
	ON graph_facts (tenant_id, project_id, valid_to);
CREATE INDEX IF NOT EXISTS idx_graph_facts_tenant_project_object_entity
	ON graph_facts (tenant_id, project_id, object_entity_id)
	WHERE object_entity_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_graph_facts_active_entity_object
	ON graph_facts (tenant_id, project_id, scope, subject_entity_id, predicate_id, object_entity_id)
	WHERE valid_to IS NULL AND object_entity_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS uq_graph_facts_active_entity_value
	ON graph_facts (tenant_id, project_id, scope, subject_entity_id, predicate_id, object_value)
	WHERE valid_to IS NULL AND object_value IS NOT NULL;
