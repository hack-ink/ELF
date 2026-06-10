CREATE TABLE IF NOT EXISTS core_memory_block_events (
	event_id uuid PRIMARY KEY,
	block_id uuid NOT NULL REFERENCES core_memory_blocks(block_id) ON DELETE CASCADE,
	attachment_id uuid NULL REFERENCES core_memory_block_attachments(attachment_id) ON DELETE SET NULL,
	tenant_id text NOT NULL,
	project_id text NOT NULL,
	actor_agent_id text NOT NULL,
	event_type text NOT NULL,
	target_agent_id text NULL,
	read_profile text NULL,
	prev_snapshot jsonb NULL,
	new_snapshot jsonb NULL,
	reason text NOT NULL,
	ts timestamptz NOT NULL DEFAULT now(),
	CONSTRAINT ck_core_memory_block_events_event_type
		CHECK (
			event_type IN (
				'block_created',
				'block_updated',
				'attachment_added',
				'attachment_removed'
			)
		)
);

CREATE INDEX IF NOT EXISTS idx_core_memory_block_events_block_ts
	ON core_memory_block_events (block_id, ts);

CREATE INDEX IF NOT EXISTS idx_core_memory_block_events_attachment_ts
	ON core_memory_block_events (attachment_id, ts);
