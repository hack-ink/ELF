# Graph Memory Postgres v1.0 Specification

Description: Canonical entity/fact temporal memory schema and invariants for PostgreSQL-backed graph memory.
Language: English only.

Purpose:
- Persist entities, aliases, temporal facts, and evidence links for ELF graph memory.
- Keep one active fact per `(tenant, project, scope, subject, predicate, value-or-entity)` combination.

Core tables:
- `graph_entities`
- `graph_entity_aliases`
- `graph_facts`
- `graph_fact_evidence`

============================================================
1. ENTITIES
============================================================

`graph_entities` columns:
- `entity_id uuid PRIMARY KEY`
- `tenant_id text NOT NULL`
- `project_id text NOT NULL`
- `canonical text NOT NULL`
- `canonical_norm text NOT NULL`
- `kind text NULL`
- `created_at timestamptz NOT NULL DEFAULT now()`
- `updated_at timestamptz NOT NULL DEFAULT now()`

Indexes:
- `UNIQUE (tenant_id, project_id, canonical_norm)`

Constraint and behavior:
- Canonical values are normalized by application helper before insert/upsert.
- Normalized canonical names allow idempotent upsert behavior across whitespace/case differences.

`graph_entity_aliases` columns:
- `alias_id uuid PRIMARY KEY`
- `entity_id uuid NOT NULL REFERENCES graph_entities(entity_id) ON DELETE CASCADE`
- `alias text NOT NULL`
- `alias_norm text NOT NULL`
- `created_at timestamptz NOT NULL DEFAULT now()`

Indexes:
- `UNIQUE (entity_id, alias_norm)`
- `INDEX (alias_norm)`

============================================================
2. FACTS
============================================================

`graph_facts` columns:
- `fact_id uuid PRIMARY KEY`
- `tenant_id text NOT NULL`
- `project_id text NOT NULL`
- `agent_id text NOT NULL`
- `scope text NOT NULL`
- `subject_entity_id uuid NOT NULL REFERENCES graph_entities(entity_id)`
- `predicate text NOT NULL`
- `object_entity_id uuid NULL REFERENCES graph_entities(entity_id)`
- `object_value text NULL`
- `valid_from timestamptz NOT NULL`
- `valid_to timestamptz NULL`
- `created_at timestamptz NOT NULL DEFAULT now()`
- `updated_at timestamptz NOT NULL DEFAULT now()`

Checks:
- Exactly one object reference per fact:
  - `(object_entity_id IS NULL AND object_value IS NOT NULL)` OR
    `(object_entity_id IS NOT NULL AND object_value IS NULL)`
- `valid_to IS NULL OR valid_to > valid_from`

Indexes:
- `(tenant_id, project_id, subject_entity_id, predicate)`
- `(tenant_id, project_id, valid_to)`
- `(tenant_id, project_id, object_entity_id) WHERE object_entity_id IS NOT NULL`
- `UNIQUE (tenant_id, project_id, scope, subject_entity_id, predicate, object_entity_id)
  WHERE valid_to IS NULL AND object_entity_id IS NOT NULL`
- `UNIQUE (tenant_id, project_id, scope, subject_entity_id, predicate, object_value)
  WHERE valid_to IS NULL AND object_value IS NOT NULL`

============================================================
3. EVIDENCE
============================================================

`graph_fact_evidence` columns:
- `evidence_id uuid PRIMARY KEY`
- `fact_id uuid NOT NULL REFERENCES graph_facts(fact_id) ON DELETE CASCADE`
- `note_id uuid NOT NULL REFERENCES memory_notes(note_id) ON DELETE CASCADE`
- `created_at timestamptz NOT NULL DEFAULT now()`

Indexes:
- `UNIQUE (fact_id, note_id)`
- `(note_id)`
- `(fact_id)`

============================================================
4. INVARIANTS
============================================================
- `graph_entities.canonical_norm` must be deterministic using:
  - trim
  - whitespace collapse to one space
  - lowercase
- An active fact is defined by: `valid_from <= now AND (valid_to IS NULL OR valid_to > now)`.
- Active duplicate prevention is enforced by partial unique indexes.
- When ingestion reintroduces a note equivalent to an existing active fact, the system reuses the existing fact row and appends additional evidence rows for the new note instead of creating another active duplicate fact row.

============================================================
5. CALL EXAMPLES
============================================================

```
canonical = normalize_entity_name("  Alice   Example ")
=> "alice example"

upsert_entity("tenant-a", "project-b", canonical, Some("person")) -> entity_id
upsert_entity_alias(entity_id, "A. Example")

insert_fact_with_evidence(
	"tenant-a",
	"project-b",
	"agent-c",
	"project_shared",
	subject_entity_id,
	"connected_to",
	Some(object_entity_id),
	None,
	now,
	None,
	&[note_id_1, note_id_2],
)

fetch_active_facts_for_subject(
	"tenant-a",
	"project-b",
	"project_shared",
	subject_entity_id,
	now,
)
```
