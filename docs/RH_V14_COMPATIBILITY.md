# RH V14 additive indexing

Status (2026-10-09): V0.6.1 direct continuation is installed and running from
78,241,606 after schema migration and a successful 1,000-block no-event canary.
The Pump static replay verified 52 approvals in one processed block. Normal-run
SQL first committed block 78,278,926 while the stopped old cursor remained at
78,241,605. Subsequent operator logs confirm cursor advancement to 81,489,309
at 08:17:54 +02:00 and automatic resume at 81,489,310. The latest stream log at
09:41:14 +02:00 reports output block 81,714,795; that is not proof of SQL commit.
Windows expand on unchanged cursors and reset to the current 300,000-block base
after advancement, capped at 8,000,000. The free plan supports 5 workers and the
actual server start timeout is 12 hours. Catch-up to V14,
V14 SQL projections, MySQL/API acceptance and
rollback verification remain release gates. Server evidence and backup identities
are in `deployments/4663/v14-v0.6.1-server-candidate.json`. Follow
`SUBSTREAMS_AGENT_RUNBOOK.md`; give the operator one server command at a time and
inspect its actual output before the next command. Keep production indexing on.
The development handoff and dated operational snapshot are in
[`RH_V14_HANDOFF.md`](RH_V14_HANDOFF.md); re-read live state before acting.

## Source identity

`deployments/4663/v14-sources.json` records deployment blocks and transaction
hashes extracted from the successful original/resume receipts in
`TagAI-contract-V2/deployments/4663/rh14-recovery`. Their SHA-256 values are
recorded. `fixtures/rh-v14-deployment-logs.json` contains only public receipt data.
`abi/v14/*.json` contains the event entries of the deployed RH ABI exports;
NutboxRouter uses the archived, deployed ABI without the later batch methods.
The V14 module uses a shared `ethabi` event decoder and named JSON payloads:
uint256/int256 values are decimal strings, addresses/bytes lowercase hexadecimal,
arrays retain order, booleans remain JSON booleans. Known malformed events fail
the block rather than silently advance past missing data.

| Contract | Address | Deployment block |
| --- | --- | ---: |
| NutboxRouter | `0xfc82178523687edd56f7474d6529a14f7655ab15` | 83,024,792 |
| Basket executor | `0xe2221f18ab7be5c830e853720966e5592f827d4d` | 83,065,587 |
| Basket hook | `0x23627bae70f110407a46ee05970771cb3b25ea88` | 83,065,617 |
| Basket router | `0x47fcc4e4396bfb306e4cb60e53d366c7caf17971` | 83,065,648 |
| Pump | `0xd72826378cb53182319f7a8b2882de4997ffc896` | 83,065,677 |
| Swap hook | `0x3e1d75fba24037123235f937a2bd19af135c60cc` | 83,065,795 |
| TradeCurationFactory | `0x73cb7a6ad01686659f72011eb952fba53fe6212b` | 83,065,912 |
| TradeRouter | `0xf64b0e841a756b32e0848b609e9a0d1646ec00d9` | 83,066,002 |
| LiquidityRouter | `0x37176232fbe6cbb87643308350c2ac8265ba2719` | 83,066,031 |

The domain backfill begins at **83,024,792**, not at Pump deployment: otherwise
53 price pools and most of the 105 route initializations would be lost. The
Pump deployment contains 52 constituent approvals. The deployed optional-pool
factory configuration contains one factory (TradeCuration); this is distinct
from the maximum of two optional pool instances allowed per token.

## Event coverage and BSC semantics

BSC reference: `tiptag-graph/src/mappingPump14.ts`, `mappingPump13.ts`,
`mappingToken13.ts`, `mappingHook13.ts`, and the TradeCuration factory/pool
mappings. Existing V9/V11 and Basket V1/V3 remain indexed.

| Family | V14 output / behavior |
| --- | --- |
| NewToken, curve trades | Existing discoveries/tokens/trades tables, version 14, creator, Pump; same-transaction and later-block dynamic discovery |
| IndexConfigured | `token_index_configs`, ordered `token_components`; target weights remain distinct from staking reward ratios |
| ComponentPairCreated / liquidity burned / tax burned | Component pair, initial allocations, dead LP amount, cumulative transfer tax, Pair metadata |
| Queue, recover, list | `tokens.listing_status` (`BONDING_CURVE`, `LISTING_PENDING`, `LISTED`), timestamps and chain positions, existing listing/Pair records |
| IPShareSubjectTransferred | Current `tokens.ipshare_subject` and immutable event |
| Nutbox links | Index community, component LP staking link, independent `token_optional_pools`; unknown optional factories retain links without fabricated pool types |
| Optional factory / constituent approval | Pump-scoped `pump_optional_pool_factories` / `pump_constituents` |
| SwapFeeCollected | Fee totals, buyback reserve and immutable event; unambiguous PoolManager receipt matches produce `UNISWAP_V4` trades |
| Buyback execution, notification, claim | `token_buyback_states`, immutable exact payload; subtract spent reserve instead of resetting it |
| PeriodSettled | `token_hook_periods` plus immutable event |
| Injection success/failure, listing fees, all static administration | Full named payload in `v14_events`, including revert reason bytes |
| New NutboxRouter | Router-scoped price pools/routes in `v14_router_*`; exact execution/admin history in `v14_events` |
| TradeRouter / LiquidityRouter | Full execution, refund, leg, configuration and liquidity event payloads in `v14_events`; do not double-count router summaries as independent Token trades |
| TradeCurationCreated / TradeClaimed | Shared Walnut pool/operation tables, `TRADE_CURATION`, `total_claimed`, `TRADECLAIM`, `trade_order_id`, `trade_harvested`; no virtual stake/TVL attributed to users |
| New Basket hook/router/executor | Existing Basket trades, operations and rebalance models with V3 masks and router attribution |

Every new static/dynamic event includes source, block hash/number/time,
transaction hash and global log index. `v14_events` retains the decoded ABI
payload; derived receipt `swap` data and factory community/owner metadata are
explicit additional fields. Business snapshots are emitted per event in chain
ordinal order. Duplicate (transaction hash, log index) logs within a block are
removed before stores and writers consume them.

### RH curve and receipt differences

RH V14 uses `a=1_624_898_729`, `b=2.5175516438e26`. The old shared indexer uses
`a=6_500_000_000`; preserve its module unchanged and use the RH coefficient only
for V14. Display spot prices follow the existing indexer's scaled exponential
approximation, not an executable swap quote.

`AntiSnipeInjected.tokensPurchased` increases the supply store even though it
emits no Trade. The enclosing Trade materializes final supply, maximum supply
and price. Publishing a maximum at the intermediate injection log would create
a false peak during a sell (the sell's Trade is emitted after fee handling).

RH uses the existing Uniswap V4 PoolManager address/signature, not BSC's
Pancake manager/signature. Match the nth fee with the nth matching pool Swap in
the entire receipt, allowing both fee-before-swap and fee-after-swap. When
counts differ, retain fee history and omit the ambiguous trade.

### Shared state ownership

The exact production registry branch already discovers all registered baskets,
including new registrar versions. V14 must not replay registry, basket-token
fee/claim or auction rows: only its new hook/router/executor writes are added.

Legacy Community/Staking/Locking/NFT/Basket-TVL factories retain their own
writers. Since V0.6.1, V14 static decoding and all three new stores depend only
on V14 events from their deployment blocks. TradeCuration discovery stores the
pool/community relationship; asset and event-ordinal owner metadata are resolved
from the exact preserved Walnut stores only in `v14_core_db_out`. The community
token is immutable, so claims resolve the same asset at their own event ordinal.
Backfill does not emit legacy domain output. A same-block Community close is
preserved when V14 factory metadata is materialized. The V14 Basket decoder has
no discovery-store input because shared registry/token events remain legacy-owned.

The complete V14 SQL output still requires the old Walnut read state at each
output block. This refactor removes legacy ancestors from new state construction;
it does not claim that provider caches are current or remove the old read-state
preparation requirement. Preserve all 64 old module hashes when assembling the
continuation and validate actual provider preparation before increasing limits.

Legacy router tables are keyed without a router address. Reusing those keys
would mix V11 and V14 state, so V14 router snapshots have isolated tables and
keys prefixed by the actual Router address.

Trade claim memberships live in `v14_trade_memberships`. Independent stores
cannot safely add to the old `users_count`: a later legacy action by the same
account would count it again. Downstream consumers must use the union views
`rh_read.walnut_account_communities` and `rh_read.walnut_account_pools`, and
count distinct members from the former. Do not add these members to
`walnut_pool_stakers`. Old SocialCuration claims remain unchanged.

### Explicit scope boundaries

Do not add generic ERC20 Transfer or DEX Swap/Sync signatures to block filters.
Holder snapshots remain Blockscout -> MySQL. Complete component-pair Swap/Sync
history, live reserves and asset metadata require an **address-scoped external
source**; this release stores component lifecycle/tax data but does not claim
complete component trade/price coverage. No incomplete opportunistic Swap scan
is presented as a full history. Basket live reserves/composition retain the
existing chain-read workflow.

BuybackRouter emits no events; its effects are recorded by Hook/Token events.
No keys, signing, keeper transactions or service mutations are performed here.

## Schema and downstream impact

Reviewed migration: `scripts/migrate-rh-v14-schema.sql`. It adds V14 tables and
bootstrap-default columns to existing tables. It changes the Walnut pool index
constraint into a unique index on positive values: two same-block legacy ratio
upserts may create temporary zero-index rows before the V14 factory writer
assigns their distinct block/log indexes. Positive indexes remain unique.
Pair token/token_index get bootstrap defaults for partial staking-link upserts.
Read-only union views are in `rh_read`, outside the sink's PK-validated `public`
schema. No table is dropped or truncated. Operator reviews SQL and backups,
then executes the migration before new indexing writes.

This repository does not yet implement the downstream V14 MySQL/API migration.
Before enabling the product, update `tiptag-server` projections for lifecycle,
components/optional pools, buyback states/events and TradeCuration fields and
membership views; preserve RH block/log/id cursors. Validate existing Token and
Walnut consumers plus the V14 API responses. BSC continues using its Graph path.

## Release construction and gates

### Selected operator plan: direct continuation before V14 deployment

On 2026-10-08 the operator selected direct unified continuation while the legacy
committed cursor is still before **H = 83,024,792**. The observed 76,050,099 cursor
is evidence only, not the final cutover boundary. No V14 SQL backfill has run.

1. Keep production running while building V0.6.1, auditing all 64 shared hashes,
   recording artifact identities, and testing the store-free static decoder.
   Assemble from `/opt/tiptag-substreams/tiptag-v11-cutover-v0.5.3.spkg` on server.
2. Verify the assembled unified output inherits the old output's effective
   initial block (including network overrides); each new submodule keeps its own
   deployment block. Never deploy the development template directly.
3. Back up the database/environment and execute the reviewed V14 schema migration
   through the operator. Prepare downstream RH sync compatibility before letting
   it ingest new-version rows; pause downstream writers at cutover if needed.
4. At the actual cutover, stop the old timer and gracefully stop its active sink.
   Read the exact old output hash's committed block/hash as C after the stop.
   Require **C < H**, with no prior V14 domain SQL writes. If C >= H, use the
   domain-backfill plan below; do not silently skip intervening V14 events.
5. Use dedicated fresh `unified_v14_cursors` and
   `unified_v14_substreams_history` tables created by the sink's setup mechanism.
   Configure the accepted continuation artifact with `START_BLOCK=C+1`,
   `MODULE_HASH_MISMATCH_POLICY=error`, and `INCREMENTAL_CURSOR_ID` equal to its
   exact new output hash. Do not copy, manufacture or merge opaque cursors.
6. Run a small bounded canary from C+1, verify legacy events and strict-hash
   restart/resume, then continue catch-up with the adaptive window. V14 stores
   initialize at their own deployment blocks; both legacy and V14 output will
   be indexed when catch-up reaches H. No separate historical V14 sink is needed.
7. Verify first V14 approvals/routes and all further release gates, including
   MySQL/API projections, before enabling V14 product behavior.

Rollback before any V14 output is committed can resume the preserved old sink
cursor in the old package: overlapping legacy writes may already have occurred
in the new sink, so first restore the pre-cutover database backup or prove and
promote an exact compatible new committed boundary via a separately reviewed
plan. Merely switching the package back to the old cursor can double-count
legacy additive writes. Retain package, environment, backup and cursor evidence.

### Domain backfill fallback / common validation

The manifest is a **development template**, not a production continuation.
It contains historical reference modules whose locally rebuilt hashes differ
from the installed production artifact. Do not start its default sink against
the existing production database or promote any old cursor into it.

1. Read the current server package path, exact SPKG checksum, active sink output
   and hash, database/cursor/history identities, cursor block/hash and services.
   Do not assume that the historical V0.5.2/V0.5.3 names in docs match the server.
2. Run local tests/build and identify `db_out`, `v14_backfill_db_out`,
   `v14_continuation_db_out` and the template SHA-256.
3. `make_v14_continuation` takes the exact production SPKG, V14 template and
   output path. It copies every existing module/binary/filter/network override
   unchanged and binds V14's merge input to the production artifact's declared
   active sink. The unified initial block is the earlier of the actual old
   output and V14 backfill, including network overrides. Missing or colliding
   modules fail the build.
4. Audit all shared hashes with `audit-continuation-compatibility.sh`, passing
   the **actual** old sink module and `v14_continuation_db_out`. Derive a domain
   package using `set_sink_module` with `v14_backfill_db_out`.
5. Use a single known-event bounded Pinax replay before any SQL backfill. Start
   with Pump deployment block 83,065,677 (stop exclusive 83,065,678); verify
   52 approvals. V0.6.1 `map_v14_static_events` is a store-free decoder probe;
   it does not replace validation of the full `v14_backfill_db_out` output.
   Router deployment and creation/trade/listing/buyback/claim
   fixtures need their own event blocks. Absence of an event is not coverage.
6. Review schema SQL, backups and additive shared-table impact. Backfill only
   V14 writes from 83,024,792 to the recorded boundary C (exclusive C+1), using
   dedicated cursor/history tables. Check processed-block ratio. Existing old
   store dependencies may prepare historical state if the exact cache is absent;
   a small output range is not a store-preparation bound.
7. Run `scripts/check-rh-v14.sql`, compare legacy fingerprints, verify restart
   with the same committed cursor, undo/reorg and MySQL/API projections.
8. At the reviewed cutover, both domains must be complete through C. Start the
   unified artifact at C+1 according to the runbook, with a reviewed fresh cursor
   plan. Never merge opaque cursors. Keep hash mismatch policy `error`.
9. Record actual server identities, the boundary and a verified rollback plan.

A repeated arbitrary historical output request is NOT safe against the same
canonical tables: pool counts, rewards, fees and volume use additive updates.
Normal resume uses the sink's committed cursor. Resetting a cursor requires
restoring a pre-backfill backup or a reviewed scoped reversal that includes all
shared aggregate contributions. Merely dropping V14 tables is not rollback.

Local PostgreSQL integration remains pending because Docker is not running.
The operator executed the server migration with COMMIT and verified all 11 new
tables have primary keys. The exact installed server V0.5.3 artifact audit also
passed (64 unchanged hashes). The startup canary had no events; the subsequent
normal run committed block 78,278,926 under the new output hash. V14 event-level
SQL checks and downstream projections still require validation after catch-up.


## Local validation evidence

The evidence below records the original V0.6.0 candidate. V0.6.1 changes V14
module hashes and needs new package identities and a fresh server audit/replay.
V0.6.1 local validation: 40 library tests and 2 assembler boundary tests passed;
`substreams build` and `substreams info` passed. Local assembly against the
repository's V0.5.2 artifact preserved all 64 shared module hashes. This does not
replace the exact server production-artifact audit. Development-template SHA-256:
`a3585cd1ae5f25a151e88274652f39f93bc1a361a32e920d6c628962b2b687c1`.
This is not a production continuation artifact. Its backfill graph has two store
stages plus one map stage; both new store stages now share levels with existing
Walnut stores rather than descending from them. Actual provider cost remains
unverified until the server probe succeeds.
No V0.6.0 V14 rows were written: both full-output probes and the Walnut-dependent
static probe failed the processed-block precondition with zero processed blocks.
Operator logs at 2026-10-08 11:05 +02:00 show legacy stages around 77,757,000 to
77,762,000 (SQL cursor 76,050,099), still before V14 deployment. Stage progress
increased during the excerpt; an unchanged SQL cursor alone was not a stall.

- `cargo test`: 39 passed, 0 failed (20 existing + 19 V14 tests).
- `substreams build`: successful, Rust 1.97.1 / Substreams 1.18.5.
- `substreams info`: validated old reference output and both V14 outputs.
- Continuation assembly with the repository V0.5.2 artifact preserved 64/64
  shared hashes, changed zero, and added 10 V14 modules.
- Deployment receipt fixtures verify 52 constituent approvals, 53 price pools,
  105 routes and one configured optional-pool factory.
- Exact template checksum/output identities are recorded in
  `deployments/4663/v14-local-validation.json`; these are local artifacts,
  explicitly not approved production identities.
