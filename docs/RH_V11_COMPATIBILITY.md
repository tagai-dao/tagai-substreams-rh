# RH V11 indexing compatibility contract

This document is a release gate. RH V11/V3 indexing extends the historical RH
dataset; it never replaces or stops indexing a previously deployed contract.
The BSC Graph mappings remain the semantic reference while Substreams is the RH
execution engine.

## Static contracts

| Domain | Legacy deployment | New deployment | First new block | Compatibility rule |
| --- | --- | --- | ---: | --- |
| Pump | `0x6c75e165e52e9c1661a75041650be2d919ee02a1` (V9) | `0x7686cbaf2dfc7000eb9b0d6de81e48c1211d2655` (V11) | 51,499,529 | Decode both; tokens from either Pump enter one dynamic token registry. |
| Swap hook | `0x5e8e2d77ce0d2e04ba058bbcecc13c7c8adb20cc` (V9) | `0x841dcad307a4444dc9e65f5709b2dc5e054c20cc` (V11) | 51,499,529 | Decode both into the same trade model, retaining the originating version/address. |
| Basket registry | `0x1f997deb6c8ac7bb4134bc7c6bf23f623cda25c6` | shared | 18,022,342 | Replay from the legacy boundary; use the registry version for V1/V3. |
| Basket hook | `0xc6c999fa94199da470a17806f04de85036f02a88` (V1) | `0x7103aa53a7de0af737d1dc1a257838f6f488aa88` (V3) | 52,400,796 | Preserve V1 trades and decode V3 trades/operations. |
| Basket router | `0xd96e197f139b78e9f74555701f699aa051e0a50e` (V1) | `0x9b5e6b7cc3661737e6a118e0d4f0f89fb1034653` (V3) | 52,400,826 | Correlate each hook only with its version-compatible router events. |
| Rebalance executor | `0x773c71be8b5e3c0c49d9576211d06e2f316aaf4a` (V1) | `0x1bca8a39021f6c65b62bbe79a59e41215cf19264` (V3) | 52,400,766 | V1 fields remain valid; V3 adds sell/buy masks. |
| Fee auction | `0xc2526404423ed03ce8d2608f5b94300f0aafa1a2` | shared | existing | Keep one event stream and associate auctions with both Basket versions. |

## New V11-only sources

| Source | Address | First block | Required coverage |
| --- | --- | ---: | --- |
| ImportHelper | `0xddf74905ad9ff90977154df960e21517f7e11aca` | 51,503,865 | Imported community creation and token/community association. |
| TagAISwapWrapper | `0x91ddcaeef99d674cddfffd1c1a204c5be8291a92` | 51,503,865 | Imported V2, V3 and V4 buy/sell paths plus Nutbox fee outcomes. |
| NutboxRouter | `0x200115d733106eca3954eaa5d1fcbc6d0efb78ae` | 51,514,371 | Price-pool and route lifecycle plus executed routes. |
| Basket V3 token deployer | `0xf29faec2428376d650d84471b4c41499342c6c5a` | 52,400,736 | Basket creation is authoritative through the shared registry. |
| IndexBrokerNFTFactory | `0x678871773b07322aa927fe5057870d1356f09676` | 52,436,657 | Factory, dynamic pool and AMM discovery. |
| NutboxCommunityFeeHook | `0x58e2bf5481fb1a21b477a469b278183bd93140cc` | 52,655,972 | Pool/community config, collection and silent injection success/failure. |

## Dynamic compatibility

- Every token discovered from either Pump stays tracked for bonding-curve and
  external-market events.
- Existing Nutbox communities and legacy staking, locking, social-curation,
  NFT-mining and Basket-TVL-mining pools remain in their current dynamic stores.
- IndexBroker factory events add pool and AMM addresses to new dynamic stores;
  they do not reuse legacy mining indexes.
- PostgreSQL tables are additive. Existing primary keys and immutable event
  ordering cannot be renumbered during a continuation deployment.
- The migration is an exact V0.4 continuation plus V11-only domain backfill
  beginning at the earliest new source block `51,499,529`, followed by a
  unified cutover at one recorded boundary. `make_additive_continuation`
  replaces every shared template module with the definition and WASM binary
  from the exact installed production package. The initial integrated V0.5
  package remains a full-replay reference and must not inherit the V0.4 cursor.
- The continuation becomes valid only after every unchanged legacy module and
  store hash is preserved, new-domain writes are independently replayable, and
  downstream immutable-event consumers use `(block, log/call index, id)` rather
  than ordering mixed old/new modules by independently generated entity indexes.
- A fresh blue/green database must replay from block `6,922,897` only if that
  continuation proof fails or an existing state/aggregate semantic must be
  rebuilt. A recent start block alone cannot reconstruct legacy dynamic stores.

## Required test gates

1. Manifest tests assert that every legacy and new static address is present.
2. ABI fixtures decode V9/V11 Pump and Token events, both swap hooks, Basket
   V1/V3 creation/trade/rebalance, all imported router types, fee-hook outcomes,
   and every IndexBroker pool/AMM feature family.
3. Stateful tests cover dynamic discovery in the same transaction and later
   transactions, deterministic indexes, duplicate logs and undo/reorg changes.
4. A bounded mainnet replay compares legacy entities before the V11 deployment
   block and validates new entities after it.
5. API projection tests run the same endpoint fixtures for BSC Graph and RH
   PostgreSQL, allowing only chain-specific addresses and timestamps to differ.

## Substreams structural limits

- A module may have at most 30 direct inputs. Count inputs whenever a domain is
  added to `substreams.yaml`; do not wait for production packaging to reveal an
  oversized output module.
- Keep the SQL output split by domain (`legacy`, Basket, Nutbox mining, V11,
  IndexBroker and imported trades). The final `db_out` accepts those partial
  `DatabaseChanges` outputs and restores global ordinal order before the sink.
- `manifests_keep_every_module_within_the_substreams_input_limit` is a mandatory
  regression test. Do not remove or weaken it when adding another module.
