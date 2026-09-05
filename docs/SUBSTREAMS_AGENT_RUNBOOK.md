# TipTag RH Substreams agent and operations runbook

This is the canonical workflow for developing, deploying, operating, and
upgrading the TipTag Robinhood Chain indexer. It is written for both operators
and future coding agents. Historical migration documents remain useful as
evidence, but this runbook takes precedence for current operations.

## 1. System boundaries

The production data path is:

```text
Robinhood Chain
  -> Pinax Firehose/Substreams
  -> substreams-sink-sql
  -> PostgreSQL canonical chain tables
  -> tiptag-server graph-data-sync
  -> MySQL API read models
  -> tagai-api / frontends
```

Token-holder snapshots are deliberately separate:

```text
Blockscout API
  -> tiptag-server token-holder-refresh
  -> MySQL holder tables
  -> tagai-api / frontends
```

Substreams does not index ERC-20 `Transfer` events for holder snapshots. Do not
route Blockscout holder data through PostgreSQL.

Responsibilities:

- This repository owns contract event decoding, dynamic-address stores,
  deterministic indexes, PostgreSQL changes, package construction, and sink
  operations.
- `tiptag-server` owns PostgreSQL-to-MySQL projections and Blockscout holder
  refreshes.
- `tagai-api` owns public API behavior and chain-specific feature flags.
- BSC Graph mappings are the semantic reference for features that are intended
  to behave identically across BSC and RH.

## 2. Current production pattern

RH production uses a bounded incremental systemd pair:

- `tiptag-unified-incremental.timer`
- `tiptag-unified-incremental.service`

The timer starts a `Type=oneshot` service ten seconds after the previous run
becomes inactive. The wrapper reads `eth_blockNumber`, reads the active SQL sink
cursor, and limits each invocation to `INCREMENTAL_MAX_BLOCKS` (normally
100,000). Its inclusive target is:

```text
min(latest - LATEST_LAG_BLOCKS, resolved_start + INCREMENTAL_MAX_BLOCKS - 1)
```

The SQL sink stop block is that target plus one because stop blocks are
exclusive. The log event includes `chainTargetBlock`, `resolvedStartBlock`,
`maxBlocks`, `targetBlock`, and `stopBlockExclusive`, so a large catch-up has an
observable boundary for every invocation. The first invocation of a changed
package can still spend time preparing stores that begin before
`resolvedStartBlock`; limiting the forward range does not remove that one-time
store preparation.

The active output must produce at least one cursor-bearing block within a batch.
If a sparse pipeline repeatedly completes without advancing its cursor, do not
silently increase or skip the start block: investigate the output/cursor design
before resuming, otherwise a relevant event beyond the capped range could be
starved.

The accepted production lag is currently 6,000 blocks. The service must not run
at the same time as the older continuous unified unit.

For a `Type=oneshot` service, use `TimeoutStartSec` to limit a run that remains
in `activating`. `RuntimeMaxSec` does not protect the activation phase. The
normal production watchdog is two hours; a deliberately large first-time
blue/green replay may use a separate service with a longer reviewed timeout.

Recommended unit values:

```ini
[Service]
Type=oneshot
TimeoutStartSec=2h
RuntimeMaxSec=infinity
TimeoutStopSec=90
```

## 3. Release identity and cursor invariants

Record all of the following for every release:

- Git commit
- manifest package version
- Rust toolchain version
- Substreams CLI version
- `substreams-sink-sql` version
- SPKG filename and SHA-256
- output module name and hash
- PostgreSQL database name
- cursor and history table names
- initial block and chosen cutover block

An SPKG checksum identifies the complete package file. A module hash identifies
the executable module graph. They are not interchangeable.

A cursor is opaque state owned by the exact module graph that created it. The
human-readable `block_num` and `block_id` are diagnostic fields, not permission
to attach the cursor to a different graph. Never manufacture or copy a cursor
unless the compatibility of all upstream modules and stores has been proved.

Normal operation always uses:

```dotenv
MODULE_HASH_MISMATCH_POLICY=error
```

`ignore` may be used exactly once in a reviewed continuation where old module
hashes and state semantics are intentionally preserved. Restore `error`
immediately after the new output identity is committed.

### 3.1 Block-filter and processed-block invariant

Processed-block cost must follow TipTag protocol activity, not general chain
activity. A production-reachable module must never use a chain-wide public
event signature as a standalone block-filter trigger. Forbidden examples
include:

- Uniswap-compatible V2, V3, or V4 `Swap` signatures;
- ERC-20 or ERC-721 `Transfer`;
- generic ownership, approval, or other standard-library events that unrelated
  contracts emit frequently.

Use an exact TipTag contract address (`evt_addr`) whenever the emitting address
is known. A dynamic-contract module may use a genuinely protocol-specific
event signature and must still reject addresses not present in its discovery
store. If an event is both public and emitted by dynamic addresses, do not
fall back to its chain-wide signature. Add reviewed exact addresses, obtain the
state from an address-scoped external API, or explicitly omit that event family.

Before a historical run, calculate its input range. During and after the run,
record `progress_total_processed_blocks` and this ratio:

```text
processed-block ratio = processedBlocks / historicalInputBlocks
```

A sparse protocol backfill approaching `1.0` is a release blocker. Stop it and
audit every block filter reachable from the selected sink module. A runtime
address check inside Rust does not reduce the cost of blocks already selected
by a broad index query.

Regression tests must reject known public signatures in the production
manifest and must verify that optional broad-scanning modules are not reachable
from `db_out`, backfill outputs, or continuation outputs.

## 4. Classify the upgrade before coding

Every contract change must be classified first.

### 4.1 Additive contract, no events yet

This is the cheapest safe path when all are true:

- the deployment block is known;
- users have not called the contract;
- the current sink has not passed the first relevant event;
- existing state semantics remain compatible.

Stop at a reviewed boundary before the first event, add the new address and
decoder, and perform a reviewed continuation. A changed output hash still needs
an explicit cursor migration plan.

### 4.2 Additive contract with historical events

If relevant events already occurred behind the production cursor, simply
replacing the package will miss them. The default upgrade path is:

1. derive a continuation package from the exact production SPKG so every
   unchanged legacy module, store, WASM binary, and hash remains intact;
2. build a domain-only backfill package containing only the new contracts and
   tables, starting at the earliest new deployment or first relevant event;
3. keep the old production continuation running while the new domain backfill
   catches up, unless the provider stream limit requires a reviewed pause;
4. bring both paths to the same exclusive stop-block boundary;
5. validate that the domain backfill is complete and does not duplicate or
   mutate legacy data;
6. construct the final unified package from the exact continuation and the
   accepted new-domain modules;
7. seed or promote only cursors whose upstream module compatibility has been
   explicitly verified, then let the unified package index all future blocks.

This pattern is called **continuation + domain backfill + unified cutover**.
It avoids replaying unchanged legacy history and is the preferred method for
future additive contract deployments.

Use a full blue/green replay into a separate database instead when existing
maps, stores, filters, indexes, aggregate semantics, or primary keys changed in
a way that cannot preserve the old upstream hashes. The available choices are
therefore:

- continuation + domain-only backfill + unified cutover (preferred for
  additive contracts); or
- full blue/green replay into a separate database (fallback for incompatible
  historical semantics).

Stopping the old sink by itself does not make a new package safe to start at a
recent block. Dynamic stores are not reconstructed from PostgreSQL. A recent
start is valid only when the old store state is inherited through unchanged
module hashes, or the new domain is fully independent and reconstructs all of
its own state from its declared initial block.

### 4.3 Replacement contract

Decide explicitly whether old data remains visible or is retired.

- If history is retained, index old and new addresses and store the originating
  version/address.
- If history is retired, back up first, stop downstream writers, delete only
  the scoped protocol rows, reset their MySQL projection cursors, and start the
  replacement at its exact first block with new cursor/history tables.

Never silently mix replacement semantics with additive compatibility.

### 4.4 Decoder/schema-only correction

Determine whether the correction affects only future events or makes existing
rows wrong. A future-only fix can continue at a cutover block. A historical
correction needs a scoped idempotent repair, a verified SQL migration, or a
blue/green replay.

## 5. Development workflow

### 5.1 Establish the chain facts

Before editing code, collect:

- chain ID and RPC endpoint;
- every static contract address;
- deployment transaction and deployment block;
- first relevant event block, which may be later than deployment;
- ABI/bytecode version;
- factory-to-child discovery relationships;
- whether older deployments remain supported;
- matching BSC mapping/entity behavior.

Addresses and initial blocks are release inputs. Do not infer them from names or
copy them from another network.

### 5.2 Implement the event path

For each new feature:

1. Add the verified ABI and generated Rust binding.
2. Add static address filters and the earliest valid `initialBlock`.
3. Decode factory events before child events.
4. Use fork-aware stores for dynamic contract discovery.
5. Handle same-transaction creation and child events.
6. Preserve log order and deterministic entity indexes.
7. Emit immutable event rows with block number/hash, transaction hash, and log
   or call index.
8. Emit mutable aggregate/state rows with reorg-safe operations.
9. Add schema columns with bootstrap-safe defaults where field-level upserts
   require them.
10. Split domain writers before any module exceeds 30 direct inputs.
11. Merge domain `DatabaseChanges` in global ordinal order in `db_out`.

### 5.3 Required tests

Add fixtures for:

- every event family and contract version;
- buy/sell and success/failure branches;
- dynamic discovery in the creation transaction and later blocks;
- duplicated logs;
- undo/reorg behavior;
- deterministic ordering and indexes;
- coexistence with every legacy address that remains supported;
- the 30-input structural limit.

Run:

```bash
cargo test
```

Do not weaken `manifests_keep_every_module_within_the_substreams_input_limit`.

### 5.4 Build and identify the package

```bash
substreams build
sha256sum <package.spkg>
substreams info <package.spkg> db_out
```

Before a continuation deployment, compare the exact production artifact with
the candidate artifact:

```bash
scripts/audit-continuation-compatibility.sh \
  <exact-production.spkg> <candidate.spkg>
```

Any changed shared module makes direct cursor promotion invalid. A successful
audit is necessary but not sufficient: new-domain historical writes and store
initialization must still pass the replay/idempotence gates.

Copy an accepted artifact without rebuilding it:

```bash
install -o tiptag -g tiptag -m 0644 \
  <source.spkg> \
  /opt/tiptag-substreams/<release.spkg>
```

Recalculate the checksum after the copy.

### 5.5 Live bounded fixture

Before a broad replay, run the smallest block range containing a known event.
The stop block is exclusive:

```bash
substreams run <package.spkg> db_out \
  --start-block <event-block> \
  --stop-block $((event_block + 1)) \
  -e robinhood.substreams.pinax.network:443 \
  --output json
```

Run it as the credential-owning service user or load the protected Pinax token
without printing it. Confirm exact table, primary key, amounts, direction,
addresses, block data, and ordering.

## 6. Database schema workflow

For a fresh database, prefer SQL sink setup so both business and system tables
are installed consistently:

```bash
substreams-sink-sql setup \
  "$DATABASE_URL" \
  <package.spkg> \
  --cursors-table <release_cursors> \
  --history-table <release_history> \
  --on-module-hash-mismatch error
```

For an in-place additive schema change, give the operator the migration path and
full SQL for review. Use `\set ON_ERROR_STOP on` and a transaction. The operator
executes production SQL.

Every sink-visible table in `public` must have a primary key. Put diagnostic
copies in another database/schema or dump file. A command such as
`CREATE TABLE public.some_backup AS ...` creates a table without a primary key
and can prevent every sink from starting.

Back up before destructive or in-place work:

```bash
docker exec tiptag-substreams-postgres \
  pg_dump -U tiptag -d <database> -Fc \
  > /root/backups/<named-timestamp>.dump

docker exec -i tiptag-substreams-postgres \
  pg_restore --list \
  < /root/backups/<dump> \
  | tail
```

## 7. Additive continuation and domain backfill

This is the standard path for a newly deployed contract whose history begins
after the current production architecture was released.

The key invariant is that the candidate is built from the **exact production
SPKG**, not from a locally rebuilt package with the same filename or source
tag. Even identical source can produce different module hashes when its build
inputs differ.

### 7.1 Build the two release artifacts

Build the additive template containing uniquely named V11 modules:

```bash
substreams build --manifest substreams.yaml
test -s tiptag-substreams-v0.5.2.spkg
```

On the production server, use the installed V0.4 artifact as the base:

```bash
cargo run --release --example make_additive_continuation -- \
  /opt/tiptag-substreams/tiptag-unified-substreams-v0.4.0.spkg \
  tiptag-substreams-v0.5.2.spkg \
  tiptag-v11-continuation-v0.5.2.spkg

cargo run --release --example set_sink_module -- \
  tiptag-v11-continuation-v0.5.2.spkg \
  v11_backfill_db_out \
  tiptag-v11-backfill-v0.5.2.spkg
```

The continuation package exposes `v11_continuation_db_out`, which merges the
exact old `db_out` with the new domain outputs. The backfill package exposes
only `v11_backfill_db_out`, so replaying it cannot emit unchanged legacy rows.

Record both package checksums and output hashes, then prove preservation:

```bash
sha256sum \
  tiptag-v11-continuation-v0.5.2.spkg \
  tiptag-v11-backfill-v0.5.2.spkg

substreams info tiptag-v11-continuation-v0.5.2.spkg v11_continuation_db_out
substreams info tiptag-v11-backfill-v0.5.2.spkg v11_backfill_db_out

scripts/audit-continuation-compatibility.sh \
  /opt/tiptag-substreams/tiptag-unified-substreams-v0.4.0.spkg \
  tiptag-v11-continuation-v0.5.2.spkg \
  db_out \
  v11_continuation_db_out
```

The audit must report zero changed shared modules. A package assembled from a
non-production V0.4 artifact is a structural test artifact only and must not be
deployed.

### 7.2 Backfill and cut over at one boundary

Let `H` be the first block of the new domain and `C` the final block committed
by the old production sink after it is stopped gracefully.

1. Back up PostgreSQL and MySQL.
2. Stop downstream RH `graph-data-sync` and `k-chart` writers.
3. Stop the production timer, let the current old sink stop gracefully, and
   record its cursor block as `C`.
4. Run SQL sink setup for the new schema and dedicated backfill cursor/history
   tables.
5. Run `v11_backfill_db_out` over `[H, C + 1)`. The stop block is exclusive.
6. Stop/restart the backfill once and prove it resumes from its dedicated
   cursor; validate known transactions, row counts, and aggregates.
7. Create fresh continuation cursor/history tables. Do **not** copy either the
   old sink cursor or the domain-backfill cursor into them.
8. Configure the production sink with `v11_continuation_db_out`,
   `START_BLOCK=C+1`, and the fresh continuation cursor/history tables.
9. Start the bounded production timer. Substreams reconstructs/reuses upstream
   store state, but SQL output begins only at `C+1`; PostgreSQL already contains
   old data through `C` and new-domain data through `C`.
10. After PostgreSQL catches up, start `graph-data-sync`, verify its block/log/id
    cursors, then start `k-chart`.

There is deliberately no opaque cursor promotion in this procedure. Two
independent output cursors cannot be combined safely. The database boundary,
fresh continuation cursor, exact old module hashes, and independently complete
new stores are what make the cutover valid.

Keep the completed backfill cursor and history tables until the rollback window
has passed. Some new-domain writers update aggregate columns with additive SQL
operations. Restarting with the same committed cursor is safe, but deleting or
resetting that cursor and replaying `[H, C + 1)` into the same rows can count
those aggregates twice. A deliberate fresh replay therefore requires either a
scoped cleanup of every new-domain row/aggregate or restoration of the
pre-backfill database backup.

### 7.3 Downstream cursor rule

Independent additive modules cannot share a sequential `entity_index` counter
with the preserved old modules. PostgreSQL primary keys still make their writes
idempotent, but mixed consumers must order immutable events by canonical chain
position:

```text
(block_number, log_index or call_index, stable event id)
```

For V11, run PostgreSQL migration
`scripts/migrate-v11-additive-cursors.sql` and the corresponding
`tiptag-server/src/db/sql/v31-rh-substreams-chain-cursors.sql` MySQL migration
before restarting downstream services. If IndexBroker is enabled on RH, also
run `tiptag-server/src/db/sql/v32-rh-index-broker-nft.sql`; it creates the
chain-scoped API read-model tables from the already-migrated BSC schema. BSC
remains on its existing The Graph entity-index cursors.

## 8. Full blue/green replay

Use this when a new release cannot safely inherit the production cursor.

Do not choose this merely because the output-module hash changed. First test
whether unchanged legacy modules can be preserved byte-for-byte and whether
new historical events can be isolated in a domain-only backfill. Full replay
is the fallback when that compatibility proof fails.

### 8.1 Preparation

1. Keep the current production service and downstream sync running.
2. Check Git status before pulling; preserve local changes.
3. Test and build the exact release artifact.
4. Check disk space with `df -h /` and database size with
   `pg_database_size(...)`.
5. Create a separate database, not merely separate cursor tables.
6. Run SQL sink setup against the new database.
7. Use unique release cursor/history table names.

Keep comfortable disk headroom. Investigate large directories before deleting
anything. Cargo `target/` is rebuildable; PostgreSQL, Docker volumes, backups,
and Graph/IPFS data are not. Delete retired Graph data only after confirming no
running or stopped service needs it and after explicit operator approval.

### 8.2 Shadow service

Use a separate environment file with no duplicated secrets. Load the protected
production environment first and the blue/green overrides second:

```ini
EnvironmentFile=/etc/tiptag-unified-substreams.env
EnvironmentFile=/etc/tiptag-vNext-backfill.env
EnvironmentFile=/opt/tiptag-substreams/.substreams.env
```

The blue/green override contains only:

```dotenv
DATABASE_URL=postgres://tiptag@127.0.0.1:5433/<green_db>?sslmode=disable
PACKAGE_PATH=/opt/tiptag-substreams/<release.spkg>
START_BLOCK=<canonical_initial_block>
STOP_BLOCK=
MODULE_HASH_MISMATCH_POLICY=error
CURSORS_TABLE=<release_cursors>
HISTORY_TABLE=<release_history>
```

Run it as a separate `Type=oneshot` bounded service. During the replay, seeing
two provider streams is expected: one old production stream and one green
stream. The green service must never point at the blue database.

### 8.3 Monitoring

Verify startup:

```bash
systemctl is-active <green-service>
journalctl -u <green-service> --since '2 minutes ago' --no-pager -o cat \
  | grep -E 'incremental_target|sinker configured|restarting_at|session initialized|resolved_start_block|ERROR|FATAL'
```

Verify progress:

```bash
journalctl -u <green-service> --no-pager -o cat \
  | grep -E 'substreams stream stats|postgres sink stats|ERROR|FATAL' \
  | tail

psql "$GREEN_DATABASE_URL" -c \
  'SELECT id, block_num, block_id FROM <release_cursors> ORDER BY block_num DESC;'
```

The SQL cursor is the last block committed to the database, often the last
block with output. It is not always the remote stage/head position. During a
large filtered replay, `last_block=None` and zero data messages can coexist with
five workers computing upstream stages.

Check all three signals before declaring a stall:

- a recent `session initialized with remote endpoint` line;
- provider control plane still shows the stream/workers;
- the process still has established PostgreSQL and remote TLS connections.

```bash
PID=$(systemctl show <service> --property=MainPID --value)
ps -p "$PID" -o pid,etime,stat,cmd
ss -ntp | grep "pid=$PID," || echo NO_ESTABLISHED_CONNECTION
```

If the provider stream is gone and no progress is received, stop the run and
restart from its committed cursor. Do not reset or delete the cursor.

### 8.4 Validation gates

Before cutover:

1. Compare legacy immutable row counts and fingerprints at fixed blocks.
2. Compare mutable legacy state with the blue database.
3. Verify each new contract with known transaction fixtures.
4. Verify new tables, aggregate totals, version/address fields, and event order.
5. Stop/restart the green sink and prove cursor resume.
6. Prove a repeated bounded request changes neither rows nor aggregates.
7. Exercise or review reorg behavior.
8. Run downstream PostgreSQL-to-MySQL projection tests.
9. Verify API response shapes against the BSC semantic reference.

### 8.5 Cutover

Cutover is a controlled write/read switch:

1. Stop the old timer so it cannot schedule a new run.
2. Allow or stop the active old run gracefully and record its reached stop
   boundary.
3. Bring green to the same reviewed exclusive stop block.
4. Re-run all reconciliation gates.
5. Stop RH `graph-data-sync` and any other PostgreSQL readers/writers.
6. Update the production environment to the green database, accepted SPKG,
   start block, cursor table, and history table.
7. Point downstream RH sync at the green PostgreSQL database.
8. Restart the production timer and confirm a strict-hash resume.
9. Restart downstream sync and verify MySQL projections.
10. Verify API and frontend fixtures.

Retain the old database, package, environment snapshot, and stopped service
until the observation window passes.

### 8.6 Rollback

Rollback must not delete green data:

1. Stop the new timer/service.
2. Stop downstream RH sync.
3. Restore the old database/package/cursor environment snapshot.
4. Start the old timer and verify its cursor/session.
5. Restart downstream sync.

MySQL rollback requirements depend on whether new-only entities were projected.
Prepare scoped cleanup SQL before cutover; never truncate unrelated TagAI,
Nutbox, Basket, or holder data.

## 9. Downstream MySQL and API work

A Substreams feature is not finished when PostgreSQL rows appear.

For each new PostgreSQL entity:

1. Add or review the MySQL schema migration.
2. Add a deterministic projection cursor in `tiptag-server`.
3. Copy immutable events incrementally and refresh mutable state idempotently.
4. Preserve imported/non-chain MySQL rows when they intentionally coexist.
5. Add API queries and response-shape tests.
6. Restart only the chain-specific services.
7. Confirm projection counts and cursor values.

Pool creation may be optimistically registered by an API only after verifying
the transaction, receipt, expected factory, emitted child address, and live
contract getters. The canonical Substreams projection must later reconcile the
same row idempotently.

Holder refresh remains a separate service. The accepted policy applies to both
TagAI and Basket tokens: active tokens are refreshed more frequently, inactive
tokens less frequently, and a daily request budget protects the Blockscout API
quota. A refresh replaces the full holder snapshot so addresses that sold their
entire balance are removed.

## 10. Operational checks

### 10.1 Service state

```bash
systemctl is-active tiptag-unified-incremental.timer
systemctl is-active tiptag-unified-incremental.service
systemctl list-timers tiptag-unified-incremental.timer --no-pager
```

An incremental service is normally `activating` while a bounded run is active.
The timer can show no next trigger until that run becomes inactive.

### 10.2 Startup and target

```bash
journalctl -u tiptag-unified-incremental.service \
  --since '10 minutes ago' --no-pager -o cat \
  | grep -E 'incremental_target|restarting_at|session initialized|resolved_start_block|ERROR|FATAL'
```

### 10.3 Cursor

```sql
SELECT id, block_num, block_id
FROM <current_cursor_table>
ORDER BY block_num DESC;
```

Calculate lag against the target printed by `incremental_target`, not against a
hard-coded handoff block.

### 10.4 Process health

A live local process does not prove a live remote stream. Conversely, zero data
messages do not prove failure during historical computation. Correlate logs,
provider workers, TCP connections, and cursor/stage progress.

### 10.5 Error scan

```bash
journalctl -u <service> --since '30 minutes ago' --no-pager -o cat \
  | grep -E 'ERROR|FATAL|retry|timeout|terminated|completed'
```

## 11. Incident recovery rules

- First reproduce a missing event with `substreams run` for exactly one block.
- Query every expected PostgreSQL event table by transaction hash.
- If direct package output contains the event but SQL does not, compare the
  request range, output hash, cursor table, block ID, and sink logs.
- Create recovery cursor/history tables with SQL sink setup; do not handcraft
  their schema.
- Before promoting a recovery cursor, save the original cursor outside the
  sink's public schema or ensure any SQL copy has a primary key.
- Promote only an exact opaque cursor returned by the same module graph.
- Restart with `MODULE_HASH_MISMATCH_POLICY=error` and verify the next block.
- Never delete immutable rows or reset the main cursor as a first response.

## 12. Current V11 release record

This section is a deployment record, not a substitute for re-verification.

As of 2026-09-04:

- source commit: `69ab342`
- manifest version: `v0.5.0`
- package: `tiptag-substreams-v0.5.0.spkg`
- package SHA-256:
  `1fc27cb2e0d65fa8b44bf3b757dc18e11ee504a4c9a0ceae0dd6c6ea6d91c41b`
- `db_out` hash: `3c386a05765c83eb9f8d137c795a203caec3f141`
- `db_out` initial block: `6,922,897`
- green database: `tiptag_rh_v11`
- green cursor table: `unified_v5_cursors`
- green history table: `unified_v5_substreams_history`
- shadow service: `tiptag-v11-backfill.service`
- shadow target policy: `latest - 6,000`

The first V11 blue/green attempt was intentionally stopped before cutover. A
module audit between V0.4 and the initial V0.5 build found that all 33 shared
application modules had different hashes (only the two imported `ethcommon`
modules were unchanged). The initial V0.5 package therefore cannot inherit the
V0.4 cursor directly.

The replacement implementation now uses `make_additive_continuation` to
preserve each shared V0.4 module definition, WASM binary, block filter, and
network-specific initial-block/parameter override. It isolates V11 Pump/Swap,
Basket V3, imported markets, Nutbox Router, Community Fee Hook, and IndexBroker
behind `v11_backfill_db_out`, and merges them only for future blocks through
`v11_continuation_db_out`. RH downstream Token cursors use canonical
block/log/id ordering. V0.4 remains production until the exact server artifact
is used to build the final packages and every backfill/cutover gate passes.

The corrected additive template currently has this development identity:

- manifest version: `v0.5.2`
- template package: `tiptag-substreams-v0.5.2.spkg`
- template SHA-256: record after the server build
- `v11_backfill_db_out` initial block: `51,499,529`

The V0.5.1 additive backfill completed `[51,499,529, 53,869,281)` but
reported `2,334,178` processed blocks for a `2,369,752`-block input range, a
`98.50%` ratio despite only a handful of TipTag transactions. The
`map_imported_trade_events` path used chain-wide V2/V3/V4 `Swap` signatures,
so unrelated DEX activity woke the graph before runtime address checks could
discard it. V0.5.1 production catch-up was stopped with its timer still
disabled. V0.5.2 replaces that block filter with the exact TagAISwapWrapper
address, so pool `Swap` logs may be inspected inside an already selected
TipTag transaction but can never select unrelated chain blocks. It also removes
ERC-721 `Transfer` as an IndexBroker block-filter trigger.

Local continuation/backfill packages assembled from the repository's V0.4
artifact are structural test artifacts only because that V0.4 checksum does not
match production. Their checksums are deliberately not release identities. The
final continuation/backfill checksums and output hashes must be generated and
recorded on the server from the exact installed production V0.4 SPKG.

V11 is an additive compatibility release: V9 and Basket V1 remain indexed.
The original integrated V0.5 graph requires a fresh blue/green replay from
block `6,922,897`; the replacement continuation design avoids that replay by
preserving V0.4 and isolating the new paths.

Contract addresses and first blocks are maintained in
`docs/RH_V11_COMPATIBILITY.md` and must be checked against deployment receipts
again before cutover.

## 13. How a future agent should collaborate with the operator

- Begin with read-only state collection.
- Give one production task per response.
- Explain whether the task is read-only, reversible, or destructive.
- For destructive work, name the exact target and recovery consequence, then
  wait for explicit approval.
- Never assume a command succeeded; inspect the operator's actual output.
- Do not move to the next gate when output differs from the expected hash,
  block, service state, database, or table set.
- Keep the working production path available until the new path is fully
  validated.
- End each release with a committed runbook update containing exact identities,
  cutover boundary, validation evidence, and rollback state.
