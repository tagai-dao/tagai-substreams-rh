# TipTag RH migration plan

## Target

The long-term path is:

`Robinhood Firehose -> Pinax Substreams -> PostgreSQL -> TipTag services/API`

This retains deterministic, fork-aware indexing while removing the paid
historical JSON-RPC workload and the Graph Node/IPFS runtime. The temporary
`graph_out` adapter exists only to keep an escape hatch for Graph Node `v0.41.2`.

## Current migration inventory

The RH subgraph has 7 static data sources, 5 dynamic templates, 27 handlers, and
19 entities (including `Counter`). No mapping performs an `eth_call`; all state
is derived from events and transaction receipts, which the RH extended block
model contains.

| Slice | Source contracts | Result | Status |
|---|---|---|---|
| 0 | Pump `0x6C75...02A1` | Decode all Pump events and prove Graph/SQL output | Live-tested |
| 1 | Pump + dynamic Token9 | `Token`, `TokenTrade`, `TokenTransfer`, `TokenHolder`, `ListedToken`, `Pair` | Implemented and live-tested |
| 2 | IPShare `0x8A7b...96d1` | `Account`, `IPShareSummary`, `Holder`, `Staker`, `Stake`, `Trade`, `ValueCaptured` | Live-tested |
| 3 | SwapHook `0x5e8e...20Cc` + PoolManager receipts | Listed-token trades and price | Implemented and reconciled |
| 4 | Walnut factories/templates | `Walnut`, `Community`, `Pool`, `UserOperationHistory` | Implemented and reconciled |
| 5 | SQL sink | PostgreSQL replacement for Graph Node | Implemented and live-tested |
| 6 | RH API reader | Token/IPShare reads through PostgreSQL with an explicit Graph rollback switch | Implemented and locally verified |

The SQL sink is live-tested locally. The RH `tagai-api` reader is implemented
behind `SUBSTREAMS_READER_ENABLED`; BSC remains on its existing Graph path and
FPMM remains on its separate endpoint. The legacy `tiptag-server`
`graph-data-sync` job does not run the RH application read path and is not
required for this cutover.

### Slices 3 and 4 verified

The first RH V4 swap at block `9491855` matches the legacy Graph amounts, fees,
direction, trader, and sqrt-price-derived price. Walnut replaces all four active
factories and their dynamic templates. At block `6922897`, the first community,
social pool, owner/DAO state, ratio, and operations 1-4 match the legacy Graph,
including the Graph runtime's static-source-before-template operation order.

### Slice 1 verified

The dynamic template replacement is implemented with two stores:

- `store_token_addresses`: remembers every Pump-created Token contract.
- `store_token_balances`: fork-aware BigInt balances keyed by Token and holder.
- `store_bonding_curve_supply`: exposes the exact supply at each Trade ordinal.
- `store_entity_indexes`: reproduces the legacy monotonic indexes deterministically.

At block `6922897` it produced and persisted:

- one Token row;
- one bonding-curve Trade matching the old Graph amounts exactly;
- three raw Transfers, of which two match the old Graph non-zero-address rule;
- three final balance rows (Token contract, Pump, and creator);
- correct buy count, cumulative fees, and bonding-curve supply.
- legacy-compatible price `6502582385`, max supply, and Token/Trade/Transfer indexes.

The price and first Trade index were compared against the existing RH Graph and
match exactly. `TokenListedToDex` decoding and SQL output are implemented; its
live fixture is block `9491747` (token `0x6419...b89d`, pool
`0x5874...1fce`).

The legacy V9 mapping assigns `sellsmanFee` from `sellTimes + event fee`, which
loses previously accrued fees. The SQL model intentionally fixes this and keeps
a true cumulative fee total; consumers that require byte-for-byte reproduction
of that legacy bug must use the immutable Trade rows instead.

The legacy `CreateIPshare` handler also builds its self-holder ID by joining two
hex strings, while the Trade handler joins address bytes. For one subject this
creates two Graph `Holder` rows for the same `(holder, subject)` pair: balances
`10000000000000000000` and `-3894583981084573184`. PostgreSQL uses the semantic
composite relation and stores one correct net balance,
`6105416018915426816`. Consequently the accepted SQL relationship count is 16
unique pairs rather than the legacy store's 17 physical rows; account relation
counts remain unchanged.

### Slice 2 verified

The IPShare contract was deployed earlier, but its first protocol event is at
block `6922897`; indexing therefore starts at that proven event boundary with:

- all four events: `CreateIPshare`, `Trade`, `ValueCaptured`, and `Stake`;
- deterministic indexes for created shares, trades, captures, and stakes;
- current account supply, fees, captured value, total stake, and relationship counts;
- fork-aware holder/staker balances and zero-crossing count updates;
- protocol summary fees and buy/sell/capture totals;
- immutable SQL history with transaction hash and log index.

Block `6922897` contains all four event types in one transaction and was used for
the live Pinax and PostgreSQL acceptance test. The IPShare/Trade/Capture/Stake
indexes, amounts, supply, fees, balances, and counts match the legacy mapping
semantics. The old Graph endpoint cannot serve a historical account query at
this block because its current graft starts later; immutable fields were
compared with its indexed entities and aggregates were checked from the same
deterministic event sequence.

## Verified fixture

- Block: `6922897`
- Block hash: `bd5cfb87bb55c8d7922d20b044bbf263f342eb77ec994c6de98011ca380aad98`
- Transaction: `b992d948ebc950e0d3d0f099fa6cfa384d8c035c00ee74484ed8529a25604561`
- Token: `0x99121234ed5e7de803dfba09d2e2d97048ca5318`
- Creator: `0x16290796f2cd9f3ee97d3dd6bddfe9557c6d9b67`
- Symbol: `rhtst`

The official `substreams codegen sql` command in CLI `1.18.5` panics on this
project's repeated event messages (`not implemented yet`). The migration uses
the official compatible `substreams-database-change 4.0.0` crate directly.

## Dynamic contract strategy

Subgraph templates become Substreams stores:

1. Decode factory creation events.
2. Store each created contract address and its type.
3. Scan block receipts once and retain logs whose address is in that store.
4. Apply state changes in deterministic stores and emit sink rows.

The SwapHook logic needs no RPC receipt lookup. The RH block input already
contains every transaction receipt, so the module can correlate
`SwapFeeCollected` with the PoolManager `Swap` log in the same transaction.

Walnut SQL preserves the nullable `chainId` distinction used by the legacy
sync: factory `ADMINCREATE`/`ADMINADDPOOL` operations remain NULL, while dynamic
community admin handlers write `0`. `ADMINSETFEE.amount` also contains the
ratio, and every active RH pool starts with `tvl = 0`, matching the AssemblyScript
mappings rather than relying on SQL NULL defaults.

## Compatibility requirements

The migration must preserve the fields and ordering actually consumed by:

- `tiptag-server/src/utils/graphql.ts`: ten monotonic `index` cursors used by
  `graph-data-sync.ts`.
- `tagai-api/src/utils/graph.js`: token holders, trades/prices, and IPShare
  account/holding reads.
- `tagai-api/routes/tiptag.js`: PCS/FPMM queries are separate and are not part of
  this RH TipTag subgraph migration.

Use explicit sequence/cursor columns in PostgreSQL. Never rely on row insertion
order. Every immutable event table includes block number, canonical block hash,
transaction hash, and log index so reconciliation and fork audits are possible.

The reader migration has complete SQL source coverage:

| Legacy reader | PostgreSQL source |
|---|---|
| token/IPShare discovery cursors | `tokens.entity_index`, `accounts.ipshare_index` |
| listings, Token trades/transfers | `token_listings`, `token_trade_events`, `token_transfer_events` |
| token holders/user balances | `token_balances` joined with `accounts`/`tokens` |
| Token price and fee reports | `tokens.price`, immutable `token_trade_events` |
| IPShare account/holding state | `accounts`, `ipshare_holders`, `ipshare_stakers` |
| IPShare trade/capture cursors | `ipshare_trade_events`, `ipshare_value_capture_events` |
| Walnut community/pool/op cursors | `walnut_communities`, `walnut_pools`, `walnut_operations` |
| Walnut membership relations | `walnut_account_communities`, `walnut_account_pools`, `walnut_pool_stakers` |

The active RH API methods for token holders, user token holdings, Token trades,
token rankings/prices, IPShare account state, and IPShare holdings now have
PostgreSQL implementations. Their public response shapes remain compatible with
the previous GraphQL helpers.

This covers the fields currently selected by
`tiptag-server/src/utils/graphql.ts`, `tagai-api/src/utils/graph.js`, and the
historical fee-report scripts. FPMM queries remain owned by `fmpp-graph` and
are intentionally outside this RH TipTag migration.

## Acceptance tests

1. Compare entity/row counts at fixed RH blocks with the current RH GraphQL
   deployment.
2. Compare a known token (`0x99121234ed5e7de803dfba09d2e2d97048ca5318`)
   including creation, trades, transfers, holders, listing, and price.
3. Compare IPShare and Walnut cursor batches returned to `graph-data-sync.ts`.
4. Replay a block range twice and prove idempotence.
5. Stop/restart the sink and prove it resumes from its cursor.
6. Exercise an undo/reorg signal and prove rolled-back rows/state disappear.
7. Run shadow indexing until the old and new stores agree before switching API
   reads.

The machine-checkable fixed-height gate is
`scripts/reconcile-fixed-15217318.sql`. It verifies legacy row counts, dense
cursor maxima, Pump/IPShare/Walnut summaries, and the account IPShare cursor;
any mismatch terminates `psql` with a non-zero status.

The gate additionally checks complete canonical field fingerprints for all 7
Walnut communities, all 7 pools, and all 33 operation-history rows. This covers
the nullable factory/dynamic `chainId` distinction, pool metadata, ownership,
DAO/treasury state, memberships, ratios, operation payloads, social-claim
fields, timestamps, and transaction hashes—not only aggregate counts.

The legacy endpoint was subsequently grafted at block `15268047`, so it can no
longer serve a historical query at `15217318`. A fresh query at block
`15269049` returned the same counters and all three protocol summaries, proving
that no TipTag event changed the captured baseline across that interval.

### Recorded acceptance evidence

- Record the final `db_out` module hash and package SHA-256 for each deployed
  artifact. Package identities from earlier Basket deployments are retired and
  must not be used to validate a replacement release.

- The sink resumed from PostgreSQL cursor `#6922897`, then committed the next
  event block `#7032580` atomically.
- Replaying `6922897:6922898` against an already-complete isolated database
  exited with `cursor reached your stop block` and flushed zero rows.
- The final package writes the canonical `0xbd5c...ad98` block hash into every
  immutable event row from the first RH fixture. The fixed-height SQL gate
  rejects missing or malformed hashes and checks this exact first-block hash.
- Before/after replay, all checked row counts and cursor values were identical;
  Token and TokenTrade row fingerprints remained
  `169ec46c9a777617f2cbe03672e7987e` and
  `9ad5198fab2bd90944aed967add77648` respectively.
- During the full backfill, the four independent cursors reached index `201`.
  TokenTrade, TokenTransfer, IPShare Trade, and ValueCaptured matched the legacy
  Graph at that exact index, including transaction, accounts, direction,
  amounts, fees, supply, and price. The strict fixed-height gate retains these
  rows as mid-history fixtures so a globally shifted cursor cannot pass merely
  by ending at the same count.
- PostgreSQL sink startup confirms `handle_reorgs: true`. A live undo-signal
  exercise remains part of the final head catch-up check; historical finalized
  blocks correctly produce no undo signal and their prunable history is not
  retained.
- `scripts/test-reorg.sql` passes the sink's documented PostgreSQL history
  model for insert, update, and delete operations: reverse-order undo restores
  the last-valid state, removes fork inserts, and prunes invalid history rows.
