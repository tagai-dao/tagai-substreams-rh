# AGENTS.md

This repository indexes TipTag/TagAI contracts on Robinhood Chain with
Substreams and writes canonical chain state and immutable events to PostgreSQL.

Before changing manifests, schemas, packages, production services, cursors, or
contract addresses, read `docs/SUBSTREAMS_AGENT_RUNBOOK.md` in full. Also read
`docs/RH_V11_COMPATIBILITY.md` when the change touches V11, Basket V3, imported
markets, Nutbox Router, Community Fee Hook, or IndexBroker.

## Non-negotiable rules

- Work with the operator one production step at a time. State the expected
  output and wait for the actual output before giving the next mutation.
- Never stop the working production indexer during development, builds, schema
  review, or a blue/green backfill. Stop it only at an approved cutover point.
- Treat the SPKG SHA-256 and output-module hash as separate release identities;
  record both, together with the Git commit and toolchain versions.
- Keep `MODULE_HASH_MISMATCH_POLICY=error` in normal operation. `ignore` is a
  one-time migration tool and requires an explicit compatibility proof.
- Never copy or reuse a cursor merely because its block number looks suitable.
  A cursor belongs to its exact module graph, output hash, database, and block.
- Do not replay overlapping history into canonical tables that use additive
  updates unless idempotence has been proved for every affected table.
- For additive contract releases, default to an exact old-package
  continuation plus domain-only historical backfill, followed by a unified
  cutover at one recorded boundary. Do not replay unchanged legacy history.
- Use a new PostgreSQL database for a full blue/green replay. Do not use a new
  cursor table in the old database as a substitute for isolation.
- Do not create ad-hoc backup tables without primary keys in the sink's
  `public` schema. The SQL sink validates every public table and will refuse to
  start when one lacks a primary key.
- Keep every Substreams module at or below 30 direct inputs. Preserve the
  structural regression test that enforces this limit.
- Do not add ERC-20 Transfer-based holder indexing to Substreams. RH holder
  snapshots come from Blockscout and are written by `tiptag-server` directly to
  MySQL.
- Do not print secrets. Pinax credentials live in
  `/opt/tiptag-substreams/.substreams.env`; database secrets live in protected
  server environment files.
- Database migrations are reviewed SQL artifacts. Show the operator the SQL
  file and impact first; the operator executes production SQL.
- Preserve unrelated local changes. `src/pb/.last_generated_hash` is generated
  build state and may change after code generation or a build.

## Required release gates

At minimum, a contract-indexing release must pass:

```bash
cargo test
substreams build
substreams info <package.spkg> db_out
sha256sum <package.spkg>
```

Use a bounded live-chain replay for every newly decoded event family. A release
is not complete until PostgreSQL rows, downstream MySQL projections, API
responses, restart/resume behavior, and rollback have been checked.
