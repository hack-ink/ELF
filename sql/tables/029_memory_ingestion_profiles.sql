CREATE TABLE IF NOT EXISTS memory_ingestion_profiles (
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	pipeline text NOT NULL,
	profile_id text NOT NULL,
	version integer NOT NULL,
	profile jsonb NOT NULL,
	created_at timestamptz NOT NULL DEFAULT now(),
	created_by text NOT NULL DEFAULT 'system',
	CONSTRAINT pk_memory_ingestion_profiles
		PRIMARY KEY (tenant_id, project_id, pipeline, profile_id, version),
	CONSTRAINT ck_memory_ingestion_profiles_pipeline
		CHECK (pipeline IN ('add_event')),
	CONSTRAINT ck_memory_ingestion_profiles_version
		CHECK (version > 0),
	CONSTRAINT ck_memory_ingestion_profiles_profile
		CHECK (jsonb_typeof(profile) = 'object')
);

CREATE INDEX IF NOT EXISTS idx_memory_ingestion_profiles_lookup
	ON memory_ingestion_profiles (tenant_id, project_id, pipeline, profile_id, version DESC);
