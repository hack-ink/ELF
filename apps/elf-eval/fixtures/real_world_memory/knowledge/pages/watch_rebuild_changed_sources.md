# Knowledge Watch/Rebuild Changed-Source Page

## Changed Source Scope

Changed-source rebuild selects pages that already cite the updated Source Library
source refs and leaves unrelated pages out of the rebuild.

Sources: `watch-source-original`, `watch-source-updated`,
`watch-source-updated-event`.

## Stale Adapter Claim

The stale claim that ELF has a contained PageIndex/OpenKB adapter pass is retained
only as lint evidence.

Sources: `watch-stale-page-snapshot`, `watch-stale-claim-detected`.

## Memory Candidate Boundary

Watch/rebuild may propose a memory candidate from page deltas, but it must not
mutate Source Library documents or Memory Notes directly.

Sources: `watch-memory-candidate-proposal`,
`watch-memory-candidate-proposed`.

## Lint Findings

- `lint-contained-adapter-pass-stale`: stale claim; the contained PageIndex/OpenKB
  adapter-pass claim conflicts with the current reference-only comparison boundary.

## Version Diff

Schema: `elf.knowledge_page.version_diff/v1`.

Changed sections: `changed-source-scope`, `stale-adapter-claim`.

Unchanged sections: `memory-candidate-boundary`.
