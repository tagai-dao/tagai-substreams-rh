# The Graph support matrix for Robinhood Chain

Checked against The Graph Network Registry on 2026-07-20.

## Confirmed

| Capability | Status | Detail |
|---|---|---|
| Chain identity | Supported | `robinhood`, EIP-155 chain ID `4663` |
| Substreams | Supported | `robinhood.substreams.pinax.network:443` |
| Firehose | Supported | `robinhood.firehose.pinax.network:443` |
| EVM block model | Extended | `sf.ethereum.type.v2.Block`, including receipts/logs/calls |
| History | Genesis | First streamable block is `0` |
| Endpoint reachability | Verified | DNS, TLS, HTTP/2 responded successfully on 2026-07-20 |
| Hosted Subgraphs | Not advertised | Registry field `services.subgraphs` is empty |
| Issuance rewards | Disabled | Registry field `issuanceRewards` is false |
| Current Graph Node SPS support | Removed | Graph Node `v0.42.0` removed Substreams-powered Subgraphs |

Registry source:
`https://networks-registry.thegraph.com/TheGraphNetworksRegistry.json`

## Important Graph Node version boundary

Graph Node `v0.42.0`, released in March 2026, removed all Substreams support.
The `graph_out` module and `subgraph.yaml` in this repository compile with Graph
CLI, but can run only on an older Graph Node such as `v0.41.2`. Pinning that old
version indefinitely is not a safe production architecture because it misses
future bug and security fixes.

The Graph's current Substreams quick start remains valid for running Rust/WASM
modules against supported endpoints and sending output to sinks. It no longer
means that current Graph Node releases can consume a Substreams-powered
Subgraph.

Official release source:
`https://github.com/graphprotocol/graph-node/releases/tag/v0.42.0`

## Architecture decision

1. Recommended: RH Substreams -> PostgreSQL SQL sink -> TipTag API/MySQL sync.
   This eliminates Graph Node, IPFS, and historical RPC ingestion.
2. Transitional only: RH Substreams -> Graph Node `v0.41.2` -> existing GraphQL.
   This minimizes application changes while the SQL/API readers are migrated.
3. Do not deploy the Substreams manifest to Graph Node `v0.42.0` or later.

The existing mappings do not perform contract `eth_call`s, so the indexed data
can be reproduced from RH's extended EVM block stream without an archive RPC.
Live chain-head or unrelated application reads may still use an inexpensive
public RPC.

## Live tests completed

1. Authentication through The Graph Market succeeded.
2. Pinax executed the local package against RH block `6819860`.
3. Pump `NewToken` was decoded at block `6922897` for token
   `0x99121234ed5e7de803dfba09d2e2d97048ca5318` (`rhtst`).
4. The same event produced valid `EntityChanges` and SQL `DatabaseChanges`.
5. SQL sink `v4.13.1` created PostgreSQL business/cursor/history tables and
   wrote the known token row.
6. Repeating the same range resumed from the stored cursor and kept one row,
   proving the first slice is idempotent.
7. Dynamic Token address and balance stores executed successfully on Pinax. The
   first Token creation transaction matched old Graph Trade/Transfer data and
   persisted aggregate Token state plus holder balances to PostgreSQL.

## Additional live tests

The PoolManager swap receipt was decoded and reconciled at block `9491855`.
Walnut's first community transaction was reconciled at block `6922897`,
including constructor events emitted before the factory creation log. Full
backfill count reconciliation and cursor/replay results are recorded in
`MIGRATION_PLAN.md` after each acceptance run.
