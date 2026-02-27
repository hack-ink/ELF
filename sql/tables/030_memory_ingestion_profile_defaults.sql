CREATE TABLE IF NOT EXISTS memory_ingestion_profile_defaults (
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	pipeline text NOT NULL,
	profile_id text NOT NULL,
	version integer NULL,
	updated_at timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT pk_memory_ingestion_profile_defaults
		PRIMARY KEY (tenant_id, project_id, pipeline),
	CONSTRAINT ck_memory_ingestion_profile_defaults_pipeline
		CHECK (pipeline IN ('add_event'))
);

CREATE INDEX IF NOT EXISTS idx_memory_ingestion_profile_defaults_lookup
	ON memory_ingestion_profile_defaults (tenant_id, project_id, pipeline);
