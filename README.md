# TipTag Substreams

Substreams-powered replacement for the TipTag `rh` subgraph on Robinhood Chain
(EIP-155 chain ID 4663).

The package replaces the RH TipTag subgraph's Pump, dynamic Token, IPShare,
SwapHook/PoolManager, and active Walnut factory/template mappings. It maintains
fork-aware state stores and writes queryable current state plus immutable event
history to PostgreSQL without an archive JSON-RPC node.

Production uses two packages so a Basket contract upgrade does not invalidate
the historical caches of the legacy protocol stream:

- `substreams.yaml`: combined development package and schema reference.
- `substreams.basket.yaml`: Basket-only production package, starting at the
  replacement deployment block `16303863`.

The legacy production service must derive a continuation SPKG from its
already-built v0.1.0 artifact. The rewrite only makes the two legacy Basket
block filters impossible; it preserves the original WASM and every non-Basket
module byte-for-byte. Do not rebuild v0.1.0 from source because its module
hashes and PostgreSQL cursor are the continuity boundary for Pump, Token,
IPShare, Swap, and Walnut indexing.

```bash
cargo run --release --example make_legacy_continuation -- \
  /opt/tiptag-substreams/tiptag-substreams-v0.1.0.spkg \
  /opt/tiptag-substreams/tiptag-legacy-continuation-v0.1.0.spkg
```

## Usage

```bash
substreams build
substreams build --manifest substreams.basket.yaml
substreams auth
substreams gui -e robinhood.substreams.pinax.network:443
```

The Pinax endpoint requires a token issued by The Graph Market. Never commit the
token; `substreams auth` stores it in the local Substreams credential store.

## Local PostgreSQL sink

```bash
docker compose up -d postgres
./bin/substreams-sink-sql setup \
  'postgres://dev:insecure@127.0.0.1:5432/tiptag_dev?sslmode=disable' \
  ./tiptag-substreams-v0.2.0.spkg

. ./.substreams.env
./bin/substreams-sink-sql run \
  'postgres://dev:insecure@127.0.0.1:5432/tiptag_dev?sslmode=disable' \
  ./tiptag-substreams-v0.2.0.spkg --start-block 6922897 \
  -e robinhood.substreams.pinax.network:443
```

The first actual protocol event is at block `6922897`, confirmed against the
legacy Graph deployment. A bounded reconciliation can use
`6922897:15217319` (stop block is exclusive).

Static contracts are filtered by address through `ethereum-common:index_events`.
Dynamic Token and Walnut templates are filtered by their event signatures and
then checked against fork-aware address stores. This keeps future factory-created
contracts automatic while avoiding WASM execution on blocks that cannot contain
a relevant event.

For production, run the same sink command under a process supervisor with a
server PostgreSQL DSN and omit `--stop-block`. The SQL sink stores
its cursor in PostgreSQL, resumes automatically, and applies undo signals when
streaming non-final blocks. Use `--final-blocks-only` if lower latency is not
required and you prefer never to ingest reversible blocks.

The production wrapper accepts `DATABASE_URL`, plus optional `START_BLOCK`,
`STOP_BLOCK`, `PACKAGE_PATH`, and `SUBSTREAMS_ENDPOINT`:

```bash
DATABASE_URL='postgres://...' ./scripts/run-sink.sh
psql "$DATABASE_URL" -f ./scripts/reconcile.sql
```

After a bounded backfill ending at the exclusive block `15217319`, run the
strict legacy baseline gate. It exits non-zero on any row-count, cursor, or
protocol-summary mismatch:

```bash
psql "$DATABASE_URL" -f ./scripts/reconcile-fixed-15217318.sql
```

The project-local SQL sink binary is ignored by Git. The tested version is
`substreams-sink-sql v4.13.1` from StreamingFast's official release.

Record the built package's `db_out` module hash and file SHA-256 with every
production deployment. Both identities change when the WASM toolchain or
module configuration changes, so an old value must not be reused as a release
expectation.

The Basket-only artifact is `tiptag-basket-substreams-v0.1.0.spkg` and its sink
module is `basket_db_out`. Run it as a second SQL sink with
`START_BLOCK=16303863`; it may share the existing PostgreSQL database because it
writes only Basket and token-holder refresh tables and owns a separate cursor.

For local acceptance, `scripts/test-reorg.sql` validates the PostgreSQL I/U/D
rollback model inside a transaction; it leaves the target database unchanged.

The local Docker database is for migration development only. Production runs
the SQL sink continuously on a server and writes to a production PostgreSQL
instance; the developer laptop is not part of the production data path.

Production systemd setup, monitoring, API cutover, and rollback instructions
are documented in [`docs/PRODUCTION.md`](docs/PRODUCTION.md).

## RH API reader

`tagai-api` can read the Substreams PostgreSQL tables through its existing
`src/utils/graph.js` interface. This keeps route response shapes stable and
leaves BSC and FPMM reads unchanged. Enable it only after the production sink
has reached head and reconciliation has passed:

```dotenv
CHAIN_ID=4663
SUBSTREAMS_READER_ENABLED=1
SUBSTREAMS_DATABASE_URL=postgres://tiptag:change-me@127.0.0.1:5432/tiptag_rh
```

Set `SUBSTREAMS_READER_ENABLED=0` and restart the RH API to roll back to Graph.

`schema.sql` gives mutable entity columns safe bootstrap defaults because the
SQL sink implements field-level updates as PostgreSQL `INSERT ... ON CONFLICT`.
PostgreSQL checks `NOT NULL` constraints on the insert candidate before applying
the conflict update, even when the target row already exists.

Optionally, you can publish your Substreams to the [Substreams Registry](https://substreams.dev).

```bash
substreams registry login         # Login to substreams.dev
substreams registry publish       # Publish your Substreams to substreams.dev
```

## Modules

All of these modules produce data filtered by these contracts:
- _pump_ at **0x6c75e165e52e9c1661a75041650be2d919ee02a1**
### `map_events`

This module gets you only events that matched.

### `map_token_events`

Decodes `Trade` and `TokenListedToDex` from Token addresses learned from Pump
`NewToken` events, including events emitted later in the creation transaction.
ERC20 `Transfer` is intentionally excluded. A separate `tiptag-server` worker
reads Blockscout and writes holder snapshots directly to business MySQL; holder
data does not pass through this Substreams package or PostgreSQL sink.

### `db_out`

Writes Pump/Pair state, Token aggregates and holder age, immutable Token and
IPShare history, account relationships, Swap trades/prices, and Walnut
communities/pools/memberships/operations with deterministic legacy indexes.

### `map_ipshare_events`

Decodes `CreateIPshare`, `Trade`, `ValueCaptured`, and `Stake`. Supporting
stores maintain holder/staker balances, zero-value transitions, relationship
counts, and legacy event indexes across reorgs.

### `map_swap_events`

Correlates the TipTag hook fee event with the RH PoolManager `Swap` log in the
same receipt. No RPC receipt request is performed.

### `map_walnut_factory_events` / `map_walnut_events`

Replaces Graph dynamic templates with an address registry store. Community,
staking, locking, and social-curation contracts are decoded; owner state,
memberships, pool state, and operation ordering reproduce the active RH graph.

## The Graph support on Robinhood Chain

The Graph network registry currently advertises:

- Substreams: `robinhood.substreams.pinax.network:443`
- Firehose: `robinhood.firehose.pinax.network:443`
- EVM model: extended, streamable from block 0
- Hosted subgraphs: none

Graph Node 0.42 removed Substreams-powered Subgraph support. The recommended
production target is therefore Substreams -> PostgreSQL -> TipTag API. A pinned
Graph Node 0.41.2 deployment is retained only as a temporary GraphQL-compatible
bridge; see `docs/THE_GRAPH_SUPPORT.md` and `docs/MIGRATION_PLAN.md`.
