//! Additive RH V14 domain. Existing production modules are preserved in the
//! release SPKG; none of the discovery stores here replace a legacy store.
use super::*;
use ethabi::{Contract, RawLog, Token};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const PUMP: &str = "0xd72826378cb53182319f7a8b2882de4997ffc896";
const HOOK: &str = "0x3e1d75fba24037123235f937a2bd19af135c60cc";
const ROUTER: &str = "0xfc82178523687edd56f7474d6529a14f7655ab15";
const FACTORY: &str = "0x73cb7a6ad01686659f72011eb952fba53fe6212b";
const TRADE_ROUTER: &str = "0xf64b0e841a756b32e0848b609e9a0d1646ec00d9";
const LIQUIDITY_ROUTER: &str = "0x37176232fbe6cbb87643308350c2ac8265ba2719";
const BASKET_HOOK: [u8; 20] = hex!("23627bae70f110407a46ee05970771cb3b25ea88");
const BASKET_ROUTER: [u8; 20] = hex!("47fcc4e4396bfb306e4cb60e53d366c7caf17971");
const BASKET_EXECUTOR: [u8; 20] = hex!("e2221f18ab7be5c830e853720966e5592f827d4d");
const ZERO: &str = "0x0000000000000000000000000000000000000000";
const PUMP_ABI: &str = include_str!("../abi/v14/RHPumpV14.json");
const TOKEN_ABI: &str = include_str!("../abi/v14/RHTokenV14.json");
const HOOK_ABI: &str = include_str!("../abi/v14/RHSwapHookV14.json");
const ROUTER_ABI: &str = include_str!("../abi/v14/NutboxRouter.json");
const FACTORY_ABI: &str = include_str!("../abi/v14/TradeCurationFactory.json");
const CLAIM_ABI: &str = include_str!("../abi/v14/TradeCuration.json");
const TRADE_ABI: &str = include_str!("../abi/v14/TagAITradeRouter.json");
const LIQUIDITY_ABI: &str = include_str!("../abi/v14/TagAILiquidityRouter.json");

type Event14 = contract::V14Event;
type Events14 = contract::V14Events;
fn abi(source: &str) -> Contract {
    Contract::load(source.as_bytes()).expect("reviewed event ABI")
}
fn payload(e: &Event14) -> Value {
    serde_json::from_str(&e.payload).expect("decoded V14 payload")
}
fn string<'a>(p: &'a Value, key: &str) -> &'a str {
    p[key].as_str().unwrap_or("")
}
fn number(p: &Value, key: &str) -> BigInt {
    parse_bigint(p[key].as_str().unwrap_or("0"))
}
fn integer(p: &Value, key: &str) -> i64 {
    p[key]
        .as_str()
        .unwrap_or("0")
        .parse()
        .expect("bounded ABI integer")
}
fn flag(p: &Value, key: &str) -> bool {
    p[key].as_bool().unwrap_or(false)
}
#[cfg(test)]
fn bytes(address: &str) -> Vec<u8> {
    hex::decode(address.trim_start_matches("0x")).expect("hex address")
}
fn token_json(t: Token) -> Value {
    match t {
        Token::Address(a) => json!(format!("0x{a:x}")),
        Token::Uint(n) => json!(n.to_string()),
        Token::Int(n) => {
            let mut b = [0u8; 32];
            n.to_big_endian(&mut b);
            json!(BigInt::from_signed_bytes_be(&b).to_string())
        }
        Token::Bool(b) => json!(b),
        Token::String(s) => json!(s),
        Token::Bytes(b) | Token::FixedBytes(b) => json!(prefixed_hex(&b)),
        Token::Array(a) | Token::FixedArray(a) | Token::Tuple(a) => {
            Value::Array(a.into_iter().map(token_json).collect())
        }
    }
}
fn decode(contract: &Contract, log: &eth::Log) -> Option<(String, Value)> {
    let topic = log.topics.first()?;
    let event = contract
        .events()
        .find(|event| event.signature().as_bytes() == topic.as_slice())?;
    // Holder snapshots are owned by the address-scoped Blockscout worker.
    if matches!(
        event.name.as_str(),
        "Transfer" | "Approval" | "Initialized" | "EIP712DomainChanged"
    ) {
        return None;
    }
    let decoded = event
        .parse_log(RawLog {
            topics: log
                .topics
                .iter()
                .map(|t| ethabi::ethereum_types::H256::from_slice(t))
                .collect(),
            data: log.data.clone(),
        })
        .expect("known contract emitted malformed event");
    let mut p = serde_json::Map::new();
    for item in decoded.params {
        p.insert(item.name, token_json(item.value));
    }
    Some((event.name.clone(), Value::Object(p)))
}
fn envelope(
    blk: &eth::Block,
    tx: &eth::TransactionTrace,
    log: &eth::Log,
    kind: String,
    p: Value,
) -> Event14 {
    Event14 {
        kind,
        source: prefixed_hex(&log.address),
        payload: p.to_string(),
        transaction_hash: Hex(&tx.hash).to_string(),
        log_index: log.block_index,
        block_number: blk.number,
        block_hash: prefixed_hex(&blk.hash),
        timestamp: blk.timestamp().seconds,
        ordinal: log.ordinal,
        transaction_from: prefixed_hex(&tx.from),
    }
}
fn normalize(mut events: Vec<Event14>) -> Vec<Event14> {
    events.sort_by_key(|e| (e.ordinal, e.log_index));
    let mut seen = BTreeSet::new();
    events.retain(|e| seen.insert((e.transaction_hash.clone(), e.log_index)));
    events
}

// One-to-one occurrence matching handles both beforeSwap and afterSwap fees.
// Extra zero-fee swaps make the receipt ambiguous: retain fees, omit the trade.
fn matching_swap<'a>(logs: &'a [eth::Log], fee_log: &eth::Log) -> Option<&'a eth::Log> {
    let pool = fee_log.topics.get(1)?;
    let fee_topic = fee_log.topics.first()?;
    let fees: Vec<_> = logs
        .iter()
        .filter(|l| {
            l.address == fee_log.address
                && l.topics.first() == Some(fee_topic)
                && l.topics.get(1) == Some(pool)
        })
        .collect();
    let swaps: Vec<_> = logs
        .iter()
        .filter(|l| {
            l.address == CL_POOL_MANAGER
                && l.topics.first().map(Vec::as_slice) == Some(SWAP_TOPIC.as_slice())
                && l.topics.get(1) == Some(pool)
                && l.data.len() >= 192
        })
        .collect();
    if fees.len() != swaps.len() {
        return None;
    }
    let position = fees
        .iter()
        .position(|l| l.block_index == fee_log.block_index)?;
    swaps.get(position).copied()
}
fn static_events(
    blk: &eth::Block,
    community_info: impl Fn(u64, &str) -> Option<(String, String)>,
) -> Events14 {
    let contracts: BTreeMap<_, _> = [
        (PUMP, PUMP_ABI),
        (HOOK, HOOK_ABI),
        (ROUTER, ROUTER_ABI),
        (FACTORY, FACTORY_ABI),
        (TRADE_ROUTER, TRADE_ABI),
        (LIQUIDITY_ROUTER, LIQUIDITY_ABI),
    ]
    .into_iter()
    .map(|(a, s)| (a, abi(s)))
    .collect();
    let mut events = Vec::new();
    for rcpt in blk.receipts() {
        for log in &rcpt.receipt.logs {
            let address = prefixed_hex(&log.address);
            let Some(contract) = contracts.get(address.as_str()) else {
                continue;
            };
            let Some((kind, mut p)) = decode(contract, log) else {
                continue;
            };
            if address == FACTORY && kind == "TradeCurationCreated" {
                let community = string(&p, "community");
                let (asset, owner) = community_info(log.ordinal, community)
                    .expect("TradeCuration community must be discovered by legacy factory store");
                p["asset"] = json!(asset);
                p["owner"] = json!(owner);
            }
            if address == HOOK && kind == "SwapFeeCollected" {
                if let Some(swap) = matching_swap(&rcpt.receipt.logs, log) {
                    let amount0 = BigInt::from_signed_bytes_be(&swap.data[0..32]);
                    let amount1 = BigInt::from_signed_bytes_be(&swap.data[32..64]);
                    let sqrt = BigInt::from_unsigned_bytes_be(&swap.data[64..96]);
                    p["swap"] = json!({"isBuy":amount1>BigInt::from(0),"tokenAmount":amount1.absolute().to_string(),"ethAmount":amount0.absolute().to_string(),"price":swap_price(&sqrt).to_string(),"logIndex":swap.block_index,"ordinal":swap.ordinal});
                }
            }
            events.push(envelope(blk, &rcpt.transaction, log, kind, p));
        }
    }
    Events14 {
        events: normalize(events),
    }
}
#[substreams::handlers::map]
fn map_v14_static_events(
    blk: eth::Block,
    communities: StoreGetString,
    owners: StoreGetString,
) -> Result<Events14, substreams::errors::Error> {
    Ok(static_events(&blk, |ordinal, community| {
        let value = communities.get_at(ordinal, community)?;
        let asset = value.strip_prefix("COMMUNITY|")?.to_string();
        Some((asset, owners.get_at(ordinal, community)?))
    }))
}
fn discovery(e: &Event14) -> Option<(String, String)> {
    let p = payload(e);
    match (e.source.as_str(), e.kind.as_str()) {
        (PUMP, "NewToken") => Some((
            string(&p, "token").into(),
            json!({"type":"TOKEN"}).to_string(),
        )),
        (FACTORY, "TradeCurationCreated") => Some((
            string(&p, "pool").into(),
            json!({"type":"TRADE_CURATION","community":p["community"],"asset":p["asset"]})
                .to_string(),
        )),
        _ => None,
    }
}
#[substreams::handlers::store]
fn store_v14_addresses(events: Events14, store: StoreSetString) {
    for e in events.events {
        if let Some((key, value)) = discovery(&e) {
            store.set(e.ordinal, key, &value);
        }
    }
}
fn dynamic_events(
    blk: &eth::Block,
    discoveries: &Events14,
    previous: impl Fn(&str) -> Option<String>,
) -> Events14 {
    let token_abi = abi(TOKEN_ABI);
    let claim_abi = abi(CLAIM_ABI);
    let mut events = Vec::new();
    for rcpt in blk.receipts() {
        for log in &rcpt.receipt.logs {
            let address = prefixed_hex(&log.address);
            let known = previous(&address).or_else(|| {
                discoveries
                    .events
                    .iter()
                    .filter(|e| e.ordinal < log.ordinal)
                    .filter_map(discovery)
                    .find(|(key, _)| key == &address)
                    .map(|(_, value)| value)
            });
            let Some(known) = known else { continue };
            let meta: Value = serde_json::from_str(&known).expect("discovery metadata");
            let a = if string(&meta, "type") == "TOKEN" {
                &token_abi
            } else {
                &claim_abi
            };
            if let Some((kind, mut p)) = decode(a, log) {
                if kind == "TradeClaimed" {
                    p["community"] = meta["community"].clone();
                    p["asset"] = meta["asset"].clone();
                }
                events.push(envelope(blk, &rcpt.transaction, log, kind, p));
            }
        }
    }
    Events14 {
        events: normalize(events),
    }
}
#[substreams::handlers::map]
fn map_v14_dynamic_events(
    blk: eth::Block,
    discoveries: Events14,
    addresses: StoreGetString,
) -> Result<Events14, substreams::errors::Error> {
    Ok(dynamic_events(&blk, &discoveries, |key| {
        addresses.get_first(key)
    }))
}
fn supply_delta(e: &Event14) -> Option<BigInt> {
    let p = payload(e);
    match e.kind.as_str() {
        "Trade" => Some(if flag(&p, "isBuy") {
            number(&p, "tokenAmount")
        } else {
            -number(&p, "tokenAmount")
        }),
        // Anti-snipe buys increase supply without emitting a Trade.
        "AntiSnipeInjected" => Some(number(&p, "tokensPurchased")),
        _ => None,
    }
}
#[substreams::handlers::store]
fn store_v14_supply(events: Events14, store: StoreAddBigInt) {
    for e in events.events {
        if let Some(delta) = supply_delta(&e) {
            store.add(e.ordinal, &e.source, delta);
        }
    }
}
#[substreams::handlers::store]
fn store_v14_components(events: Events14, store: StoreSetString) {
    for e in events.events {
        if e.kind == "ComponentPairCreated" {
            let p = payload(&e);
            store.set(
                e.ordinal,
                string(&p, "pair"),
                &format!("{}:{}", e.source, string(&p, "asset")),
            );
        }
    }
}
fn rh_price(supply: &BigInt) -> BigInt {
    // RH V14 retains a=1_624_898_729, unlike the older indexer's 6.5 gwei.
    let exponent = supply.to_string().parse::<f64>().expect("supply f64")
        / 251_755_164_380_000_000_000_000_000.0;
    BigInt::from(1_624_898_729u64) * BigInt::from((exponent.exp() * 100_000_000.0) as u64)
        / BigInt::from(100_000_000u64)
}
fn metadata(row: &mut substreams_database_change::tables::Row, e: &Event14) {
    row.set("block_number", e.block_number)
        .set("block_hash", &e.block_hash)
        .set("block_timestamp", e.timestamp)
        .set("transaction_hash", &e.transaction_hash)
        .set("log_index", e.log_index);
}
fn init_account(t: &mut Tables, address: &str, e: &Event14) {
    if address.is_empty() || address == ZERO {
        return;
    }
    t.upsert_row("accounts", address)
        .set_if_null("joined_at", e.timestamp)
        .set_if_null(
            "entity_index",
            additive_entity_index(e.block_number, e.log_index),
        );
}
fn trade(t: &mut Tables, e: &Event14, p: &Value, token: &str, venue: &str, price: &BigInt) {
    let buy = flag(p, "isBuy");
    let idx = additive_entity_index(e.block_number, e.log_index);
    let buyer = if venue == "BONDING_CURVE" {
        string(p, "buyer")
    } else {
        &e.transaction_from
    };
    let sellsman = if venue == "BONDING_CURVE" {
        string(p, "sellsman")
    } else {
        ZERO
    };
    t.upsert_row("tokens", token)
        .add(if buy { "buy_times" } else { "sell_times" }, 1)
        .set("price", price)
        .set("price_updated_block", e.block_number)
        .set("price_updated_log_index", e.log_index);
    let row = t.upsert_row(
        "token_trade_events",
        event_id(&e.transaction_hash, e.log_index),
    );
    row.set("entity_index", idx)
        .set("token", token)
        .set("buyer", buyer)
        .set("sellsman", sellsman)
        .set("is_buy", buy)
        .set("token_amount", number(p, "tokenAmount"))
        .set("eth_amount", number(p, "ethAmount"))
        .set("tiptag_fee", number(p, "tiptagFee"))
        .set("sellsman_fee", number(p, "sellsmanFee"))
        .set("buyback_fee", number(p, "buybackFee"))
        .set("venue", venue)
        .set("pool", string(p, "poolId"))
        .set("price", price);
    metadata(row, e);
    init_account(t, buyer, e);
    init_account(t, sellsman, e);
}
fn listing(t: &mut Tables, e: &Event14, token: &str, status: &str) {
    t.upsert_row("tokens", token)
        .set("listing_status", status)
        .set("listing_state_block", e.block_number)
        .set("listing_state_log_index", e.log_index)
        .set(
            "listing_queued_at",
            if status == "LISTING_PENDING" {
                e.timestamp
            } else {
                0
            },
        );
}

fn write_event(t: &mut Tables, e: &Event14, supply: Option<BigInt>, component: Option<String>) {
    let p = payload(e);
    let idx = additive_entity_index(e.block_number, e.log_index);
    let id = event_id(&e.transaction_hash, e.log_index);
    let token = if e.source == PUMP || e.source == HOOK {
        string(&p, "token")
    } else {
        &e.source
    };
    let row = t.create_row("v14_events", &id);
    row.set("event_type", &e.kind)
        .set("source", &e.source)
        .set("payload", &e.payload);
    metadata(row, e);
    match e.kind.as_str() {
        "NewToken" => {
            let creator = string(&p, "creator");
            t.upsert_row("tokens", token)
                .set("entity_index", idx)
                .set("symbol", string(&p, "tick"))
                .set("creator", creator)
                .set("ipshare_subject", creator)
                .set("pump", PUMP)
                .set("version", 14)
                .set("creation_block", e.block_number)
                .set("creation_log_index", e.log_index);
            listing(t, e, token, "BONDING_CURVE");
            let row = t.upsert_row("pump_token_discoveries", token);
            row.set("token", token)
                .set("symbol", string(&p, "tick"))
                .set("creator", creator)
                .set("pump", PUMP)
                .set("version", 14);
            metadata(row, e);
            t.upsert_row("pump_summary", "pump").add("token_counts", 1);
            t.upsert_row("token_index_configs", token)
                .set("token", token)
                .set("entity_index", idx);
            t.upsert_row("token_buyback_states", token)
                .set("token", token);
            init_account(t, creator, e);
        }
        "IndexConfigured" => {
            let assets = p["constituentAssets"].as_array().expect("assets");
            let weights = p["targetWeights"].as_array().expect("weights");
            assert_eq!(assets.len(), weights.len());
            t.upsert_row("token_index_configs", token)
                .set("name", string(&p, "name"))
                .set("symbol", string(&p, "symbol"))
                .set("basket_fee_bps", integer(&p, "basketFeeBps"))
                .set("creator_share_bps", integer(&p, "creatorShareBps"))
                .set(
                    "retain_community_ownership",
                    flag(&p, "retainCommunityOwnership"),
                )
                .set("component_count", assets.len() as i64);
            for (i, asset) in assets.iter().enumerate() {
                let asset = asset.as_str().unwrap();
                t.upsert_row("token_components", format!("{token}:{asset}"))
                    .set("token", token)
                    .set("asset", asset)
                    .set("position", i as i64)
                    .set("target_weight", weights[i].as_str().unwrap());
            }
        }
        "NutboxLinked" => {
            t.upsert_row("token_index_configs", token)
                .set("community", string(&p, "community"));
        }
        "NutboxStakingPoolLinked" => {
            let pair = string(&p, "lpToken");
            let pool = string(&p, "pool");
            t.upsert_row("pairs", pair)
                .set("token", token)
                .set("staking_pool", pool)
                .set("staking_reward_ratio", integer(&p, "rewardRatio"));
            if let Some(c) = component {
                t.upsert_row("token_components", c)
                    .set("staking_pool", pool)
                    .set("staking_reward_ratio", integer(&p, "rewardRatio"));
            }
        }
        "NutboxOptionalPoolLinked" => {
            let row = t.upsert_row(
                "token_optional_pools",
                format!("{}:{}", token, string(&p, "pool")),
            );
            row.set("token", token)
                .set("pool", string(&p, "pool"))
                .set("factory", string(&p, "factory"))
                .set("reward_ratio", integer(&p, "rewardRatio"));
            metadata(row, e);
        }
        "OptionalPoolFactorySet" => {
            let row = t.upsert_row(
                "pump_optional_pool_factories",
                format!("{}:{}", PUMP, string(&p, "factory")),
            );
            row.set("pump", PUMP)
                .set("factory", string(&p, "factory"))
                .set("name", string(&p, "name"))
                .set("max_reward_ratio", integer(&p, "maxRewardRatio"))
                .set("enabled", flag(&p, "enabled"));
            metadata(row, e);
        }
        "ConstituentApprovalSet" => {
            let row = t.upsert_row(
                "pump_constituents",
                format!("{}:{}", PUMP, string(&p, "asset")),
            );
            row.set("pump", PUMP)
                .set("asset", string(&p, "asset"))
                .set("approved", flag(&p, "approved"));
            metadata(row, e);
        }
        "IndexTokenCreated" => {
            let index = string(&p, "indexToken");
            t.upsert_row("token_index_configs", token)
                .set("index_token", index);
            t.upsert_row("token_buyback_states", token)
                .set("index_token", index);
            t.upsert_row("baskets", index).set("source_token", token);
        }
        "FailedListingRecovered" => listing(t, e, token, "BONDING_CURVE"),
        "TokenListingQueued" => listing(t, e, token, "LISTING_PENDING"),
        "TokenListedToDex" => {
            assert_eq!(token, string(&p, "token"));
            listing(t, e, token, "LISTED");
            t.upsert_row("tokens", token)
                .set("listed", true)
                .set("listed_at", e.timestamp);
            t.upsert_row("pump_summary", "pump").add("listed_counts", 1);
            let row = t.upsert_row("token_listings", token);
            row.set("entity_index", idx)
                .set("event_token", token)
                .set("pool_id", string(&p, "poolId"))
                .set("sqrt_price_x96", number(&p, "sqrtPriceX96"));
            metadata(row, e);
            t.upsert_row("pairs", string(&p, "poolId"))
                .set("token", token)
                .set("token_index", 1)
                .set("quote_asset", ZERO)
                .set("venue", "UNISWAP_V4");
        }
        "ComponentPairCreated" => {
            let asset = string(&p, "asset");
            let pair = string(&p, "pair");
            let key = format!("{token}:{asset}");
            t.upsert_row("token_components", &key)
                .set("token", token)
                .set("asset", asset)
                .set("pair", pair)
                .set("target_weight", integer(&p, "weight"));
            t.upsert_row("pairs", pair)
                .set("token", token)
                .set("token_index", if token < asset { 0 } else { 1 })
                .set("component", key)
                .set("quote_asset", asset)
                .set("venue", "UNISWAP_V2_COMPONENT");
        }
        "ComponentLiquidityBurned" => {
            t.upsert_row(
                "token_components",
                format!("{token}:{}", string(&p, "asset")),
            )
            .set("initial_native_budget", number(&p, "nativeBudget"))
            .set("initial_token_amount", number(&p, "tokenAmount"))
            .set("initial_asset_amount", number(&p, "assetAmount"))
            .set("initial_lp_to_dead", number(&p, "liquidity"));
        }
        "ComponentPoolTaxBurned" => {
            let key = component.expect("taxed component pair must be discovered");
            t.upsert_row("token_components", key)
                .add("total_tax_to_dead", number(&p, "taxAmount"));
        }
        "Trade" => {
            let supply = supply.expect("V14 supply at trade ordinal");
            let price = rh_price(&supply);
            t.upsert_row("tokens", token)
                .set("bonding_curve_supply", &supply)
                .max("max_bonding_curve_supply", &supply)
                .add("tiptag_fee", number(&p, "tiptagFee"))
                .add("sellsman_fee", number(&p, "sellsmanFee"));
            trade(t, e, &p, token, "BONDING_CURVE", &price);
        }
        // Injection is emitted inside fee handling, before the enclosing Trade.
        // Only Trade publishes the final supply/max/price; a sell's injection
        // otherwise creates a false transient historical maximum.
        "AntiSnipeInjected" => {}
        "IPShareSubjectTransferred" => {
            t.upsert_row("tokens", token)
                .set("ipshare_subject", string(&p, "newSubject"));
        }
        "SwapFeeCollected" => {
            t.upsert_row("token_buyback_states", token)
                .add("native_reserve", number(&p, "buybackFee"))
                .add("total_buyback_fee_native", number(&p, "buybackFee"));
            t.upsert_row("tokens", token)
                .add("tiptag_fee", number(&p, "platformFee"))
                .add("sellsman_fee", number(&p, "deployerFee"));
            if !p["swap"].is_null() {
                let mut q = p["swap"].clone();
                q["tiptagFee"] = p["platformFee"].clone();
                q["sellsmanFee"] = p["deployerFee"].clone();
                q["buybackFee"] = p["buybackFee"].clone();
                q["poolId"] = p["poolId"].clone();
                trade(t, e, &q, token, "UNISWAP_V4", &number(&q, "price"));
            }
        }
        "PoolRegistered" => {
            t.upsert_row("pairs", string(&p, "poolId"))
                .set("token", token)
                .set("token_index", 1)
                .set("venue", "UNISWAP_V4")
                .set("quote_asset", ZERO);
        }
        "BuybackExecuted" => {
            t.upsert_row("token_buyback_states", token)
                .set("index_token", string(&p, "indexToken"))
                .sub("native_reserve", number(&p, "bnbIn"))
                .add("total_native_spent", number(&p, "bnbIn"))
                .add("total_index_bought", number(&p, "indexOut"));
        }
        "BuybackRewardNotified" => {
            t.upsert_row("token_buyback_states", token)
                .set("index_token", string(&p, "indexToken"))
                .set("acc_reward_per_token", number(&p, "accRewardPerToken"))
                .add("total_index_notified", number(&p, "amount"));
        }
        "BuybackRewardClaimed" => {
            t.upsert_row("token_buyback_states", token)
                .add("total_index_claimed", number(&p, "amount"));
        }
        "PeriodSettled" => {
            let row = t.upsert_row(
                "token_hook_periods",
                format!("{token}:{}", string(&p, "settledPeriodIndex")),
            );
            row.set("token", token)
                .set("period_index", integer(&p, "settledPeriodIndex"))
                .set("period_volume", number(&p, "periodVolume"))
                .set("lookup_volume", number(&p, "lookupVolume"))
                .set("ratio_ppm", integer(&p, "ratioPpm"))
                .set("inject_amount", number(&p, "injectAmount"));
            metadata(row, e);
        }
        "TradeCurationCreated" => {
            let pool = string(&p, "pool");
            let community = string(&p, "community");
            let owner = string(&p, "owner");
            let asset = string(&p, "asset");
            t.upsert_row("walnut_pools", pool)
                .set("entity_index", idx)
                .set("pool_factory", FACTORY)
                .set("community", community)
                .set("name", string(&p, "name"))
                .set("asset", asset)
                .set("created_at", e.timestamp)
                .set(
                    "status",
                    if flag(&p, "closedInBlock") {
                        "CLOSED"
                    } else {
                        "OPENED"
                    },
                )
                .set("pool_type", "TRADE_CURATION")
                .set("total_amount", 0)
                .set("tvl", 0)
                .set("total_claimed", 0);
            t.upsert_row("walnut_communities", community)
                .add("pools_count", 1);
            t.upsert_row("walnut_summary", "walnut")
                .add("total_pools", 1);
            let row = t.create_row("walnut_operations", &id);
            row.set("entity_index", idx)
                .set("operation_type", "ADMINADDPOOL")
                .set("community", community)
                .set("pool_factory", FACTORY)
                .set("pool", pool)
                .set("account", owner);
            metadata(row, e);
            init_account(t, owner, e);
            t.upsert_row("accounts", owner)
                .add("walnut_operation_count", 1);
            t.upsert_row("walnut_communities", community)
                .add("operation_count", 1);
        }
        "TradeClaimed" => {
            let account = string(&p, "user");
            let community = string(&p, "community");
            t.upsert_row("walnut_pools", &e.source)
                .add("total_claimed", number(&p, "amount"));
            let row = t.create_row("walnut_operations", &id);
            row.set("entity_index", idx)
                .set("operation_type", "TRADECLAIM")
                .set("community", community)
                .set("pool_factory", FACTORY)
                .set("pool", &e.source)
                .set("account", account)
                .set("asset", string(&p, "asset"))
                .set("amount", number(&p, "amount"))
                .set("trade_order_id", number(&p, "orderId"))
                .set("trade_harvested", flag(&p, "harvested"));
            metadata(row, e);
            init_account(t, account, e);
            t.upsert_row("accounts", account)
                .add("walnut_operation_count", 1);
            t.upsert_row("walnut_communities", community)
                .add("operation_count", 1);
            // Separate relations prevent independent legacy stores from counting
            // the same community member again. Read the union view downstream.
            t.upsert_row("v14_trade_memberships", format!("{}:{account}", e.source))
                .set("pool", &e.source)
                .set("community", community)
                .set("account", account)
                .set_if_null("created_at", e.timestamp);
        }
        _ => {}
    }
    if e.source == ROUTER {
        write_router(t, e, &p);
    }
}
fn write_router(t: &mut Tables, e: &Event14, p: &Value) {
    match e.kind.as_str() {
        "PricePoolAdded" | "PricePoolReplaced" | "PricePoolRemoved" => {
            let row = t.upsert_row(
                "v14_router_price_pools",
                format!("{}:{}", e.source, string(p, "poolId")),
            );
            row.set("router", &e.source)
                .set("pool_id", string(p, "poolId"))
                .set("active", e.kind != "PricePoolRemoved");
            if e.kind != "PricePoolRemoved" {
                row.set("token0", string(p, "token0"))
                    .set("token1", string(p, "token1"))
                    .set(
                        "source_type",
                        integer(
                            p,
                            if e.kind == "PricePoolAdded" {
                                "sourceType"
                            } else {
                                "newSourceType"
                            },
                        ),
                    )
                    .set("source_data", string(p, "sourceData"));
            }
            metadata(row, e);
        }
        "RouteAdded" | "RouteReplaced" | "RouteRemoved" => {
            let row = t.upsert_row(
                "v14_router_routes",
                format!(
                    "{}:{}:{}",
                    e.source,
                    string(p, "token0"),
                    string(p, "token1")
                ),
            );
            row.set("router", &e.source)
                .set("token0", string(p, "token0"))
                .set("token1", string(p, "token1"))
                .set("active", e.kind != "RouteRemoved");
            if e.kind != "RouteRemoved" {
                row.set("route_hash", string(p, "routeHash"))
                    .set("pool_ids", p["poolIds"].to_string());
            }
            metadata(row, e);
        }
        _ => {}
    }
}
fn event_changes(
    e: &Event14,
    supply: Option<BigInt>,
    component: Option<String>,
) -> DatabaseChanges {
    let mut tables = Tables::new();
    write_event(&mut tables, e, supply, component);
    let mut changes = tables.to_database_changes();
    for c in &mut changes.table_changes {
        c.ordinal = e.ordinal;
    }
    changes
}
#[substreams::handlers::map]
fn v14_core_db_out(
    statics: Events14,
    dynamic: Events14,
    supply: StoreGetBigInt,
    components: StoreGetString,
    legacy_community_events: contract::WalnutEvents,
) -> Result<DatabaseChanges, substreams::errors::Error> {
    let mut output = DatabaseChanges::default();
    for mut e in normalize(statics.events.into_iter().chain(dynamic.events).collect()) {
        if e.kind == "TradeCurationCreated" {
            let mut p = payload(&e);
            if legacy_community_events.events.iter().any(|old| {
                old.kind == "ADMINCLOSEPOOL" && prefixed_hex(&old.pool) == string(&p, "pool")
            }) {
                p["closedInBlock"] = json!(true);
                e.payload = p.to_string();
            }
        }
        let p = payload(&e);
        let pair = if e.kind == "NutboxStakingPoolLinked" {
            string(&p, "lpToken")
        } else {
            string(&p, "pair")
        };
        output.table_changes.extend(
            event_changes(
                &e,
                supply.get_at(e.ordinal, &e.source),
                components.get_at(e.ordinal, pair),
            )
            .table_changes,
        );
    }
    Ok(output)
}

// Registry, basket-token fee events and auctions are already owned by the
// exact legacy graph. Only new hook/router/executor emissions belong here.
#[substreams::handlers::map]
fn map_v14_basket_events(
    blk: eth::Block,
    addresses: StoreGetInt64,
) -> Result<contract::BasketEvents, substreams::errors::Error> {
    let mut events = map_basket_events_for(
        &blk,
        &contract::BasketRegistryEvents::default(),
        &addresses,
        BasketEventSelection {
            hooks: &[BASKET_HOOK],
            routers: &[BASKET_ROUTER],
            include_fee_auction: false,
            include_v1_rebalances: false,
            include_v3_rebalances: false,
            include_token_events: false,
        },
    );
    for rcpt in blk.receipts() {
        for log in &rcpt.receipt.logs {
            if log.address == BASKET_EXECUTOR {
                if let Some(e) =
                    abi::basket_rebalance_v3::events::BasketRebalanced::match_and_decode(log)
                {
                    events.rebalances.push(contract::BasketRebalance {
                        evt_tx_hash: Hex(&rcpt.transaction.hash).to_string(),
                        evt_index: log.block_index,
                        evt_block_time: Some(blk.timestamp().clone()),
                        evt_block_number: blk.number,
                        evt_ordinal: log.ordinal,
                        basket: e.basket,
                        nav_before: e.nav_before.to_string(),
                        nav_after: e.nav_after.to_string(),
                        evt_block_hash: blk.hash.clone(),
                        sell_mask: e.sell_mask.to_i32() as u32,
                        buy_mask: e.buy_mask.to_i32() as u32,
                    });
                }
            }
        }
    }
    Ok(events)
}
fn basket_changes(events: contract::BasketEvents) -> DatabaseChanges {
    let mut pieces = Vec::new();
    for e in events.trades {
        pieces.push((
            e.evt_ordinal,
            contract::BasketEvents {
                trades: vec![e],
                ..Default::default()
            },
        ));
    }
    for e in events.operations {
        pieces.push((
            e.evt_ordinal,
            contract::BasketEvents {
                operations: vec![e],
                ..Default::default()
            },
        ));
    }
    for e in events.rebalances {
        pieces.push((
            e.evt_ordinal,
            contract::BasketEvents {
                rebalances: vec![e],
                ..Default::default()
            },
        ));
    }
    pieces.sort_by_key(|(ordinal, _)| *ordinal);
    let mut output = DatabaseChanges::default();
    for (ordinal, piece) in pieces {
        let mut tables = Tables::new();
        write_basket_changes(
            &mut tables,
            contract::BasketRegistryEvents::default(),
            piece,
        );
        let mut changes = tables.to_database_changes();
        for c in &mut changes.table_changes {
            c.ordinal = ordinal;
        }
        output.table_changes.extend(changes.table_changes);
    }
    output
}
#[substreams::handlers::map]
fn v14_basket_db_out(
    events: contract::BasketEvents,
) -> Result<DatabaseChanges, substreams::errors::Error> {
    Ok(basket_changes(events))
}
#[substreams::handlers::map]
fn v14_backfill_db_out(
    core: DatabaseChanges,
    basket: DatabaseChanges,
) -> Result<DatabaseChanges, substreams::errors::Error> {
    Ok(merge_database_changes([core, basket]))
}
#[substreams::handlers::map]
fn v14_continuation_db_out(
    production: DatabaseChanges,
    v14: DatabaseChanges,
) -> Result<DatabaseChanges, substreams::errors::Error> {
    Ok(merge_database_changes([production, v14]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethabi::{
        ethereum_types::{H160, U256},
        ParamType,
    };
    use substreams_database_change::pb::sf::substreams::sink::database::v1::table_change::PrimaryKey;
    const TOKEN: &str = "0x1111111111111111111111111111111111111111";
    const ASSET: &str = "0x2222222222222222222222222222222222222222";
    const POOL: &str = "0x3333333333333333333333333333333333333333";
    const USER: &str = "0x4444444444444444444444444444444444444444";
    fn addr(a: &str) -> Token {
        Token::Address(H160::from_slice(&bytes(a)))
    }
    fn uint(n: u64) -> Token {
        Token::Uint(U256::from(n))
    }
    fn fixture(source: &str, abi_str: &str, name: &str, args: Vec<Token>, index: u32) -> eth::Log {
        let a = abi(abi_str);
        let event = a.event(name).unwrap();
        assert_eq!(event.inputs.len(), args.len());
        let mut topics = vec![event.signature().as_bytes().to_vec()];
        let mut data = vec![];
        for (p, t) in event.inputs.iter().zip(args) {
            if p.indexed {
                topics.push(ethabi::encode(&[t]));
            } else {
                data.push(t);
            }
        }
        eth::Log {
            address: bytes(source),
            topics,
            data: ethabi::encode(&data),
            block_index: index,
            index,
            ordinal: index as u64 + 100,
            ..Default::default()
        }
    }
    fn block(logs: Vec<eth::Log>) -> eth::Block {
        eth::Block {
            number: 83_100_000,
            hash: vec![7; 32],
            header: Some(eth::BlockHeader {
                timestamp: Some(prost_types::Timestamp {
                    seconds: 1_791_440_000,
                    nanos: 0,
                }),
                ..Default::default()
            }),
            transaction_traces: vec![eth::TransactionTrace {
                from: bytes(USER),
                hash: vec![9; 32],
                status: eth::TransactionTraceStatus::Succeeded as i32,
                receipt: Some(eth::TransactionReceipt {
                    logs,
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        }
    }
    fn event(source: &str, kind: &str, p: Value, index: u32) -> Event14 {
        Event14 {
            source: source.into(),
            kind: kind.into(),
            payload: p.to_string(),
            transaction_hash: "99".repeat(32),
            log_index: index,
            block_number: 83_100_000,
            block_hash: "77".repeat(32),
            timestamp: 1_791_440_000,
            ordinal: index as u64 + 100,
            transaction_from: USER.into(),
        }
    }
    fn field(changes: &DatabaseChanges, table: &str, key: &str, name: &str) -> Option<String> {
        changes
            .table_changes
            .iter()
            .find(|c| c.table == table && c.primary_key == Some(PrimaryKey::Pk(key.into())))?
            .fields
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.value.clone())
    }
    fn default_token(p: &ParamType) -> Token {
        match p {
            ParamType::Address => addr(TOKEN),
            ParamType::Uint(_) => uint(42),
            ParamType::Int(_) => Token::Int(U256::from(42)),
            ParamType::Bool => Token::Bool(true),
            ParamType::String => Token::String("fixture".into()),
            ParamType::Bytes => Token::Bytes(vec![1, 2]),
            ParamType::FixedBytes(n) => Token::FixedBytes(vec![1; *n]),
            ParamType::Array(t) => Token::Array(vec![default_token(t)]),
            ParamType::FixedArray(t, n) => {
                Token::FixedArray((0..*n).map(|_| default_token(t)).collect())
            }
            ParamType::Tuple(ts) => Token::Tuple(ts.iter().map(default_token).collect()),
        }
    }
    #[test]
    fn every_v14_event_family_decodes_and_holder_events_are_excluded() {
        for source in [
            PUMP_ABI,
            TOKEN_ABI,
            HOOK_ABI,
            ROUTER_ABI,
            FACTORY_ABI,
            CLAIM_ABI,
            TRADE_ABI,
            LIQUIDITY_ABI,
        ] {
            let a = abi(source);
            for e in a.events() {
                let log = fixture(
                    TOKEN,
                    source,
                    &e.name,
                    e.inputs.iter().map(|p| default_token(&p.kind)).collect(),
                    1,
                );
                let result = decode(&a, &log);
                if matches!(
                    e.name.as_str(),
                    "Transfer" | "Approval" | "Initialized" | "EIP712DomainChanged"
                ) {
                    assert!(result.is_none());
                } else {
                    let (name, p) = result.unwrap();
                    assert_eq!(name, e.name);
                    for arg in &e.inputs {
                        assert!(!p[&arg.name].is_null(), "{}.{}", name, arg.name);
                    }
                }
            }
        }
    }
    #[test]
    fn creation_transaction_tracks_child_only_after_discovery_and_deduplicates() {
        let new = fixture(
            PUMP,
            PUMP_ABI,
            "NewToken",
            vec![Token::String("TEST".into()), addr(TOKEN), addr(USER)],
            2,
        );
        let trade = |i| {
            fixture(
                TOKEN,
                TOKEN_ABI,
                "Trade",
                vec![
                    addr(USER),
                    addr(USER),
                    Token::Bool(true),
                    uint(100),
                    uint(200),
                    uint(3),
                    uint(4),
                ],
                i,
            )
        };
        let blk = block(vec![trade(1), new, trade(3), trade(3)]);
        let statics = static_events(&blk, |_, _| None);
        assert_eq!(statics.events.len(), 1);
        let events = dynamic_events(&blk, &statics, |_| None);
        assert_eq!(events.events.len(), 1);
        assert_eq!(events.events[0].log_index, 3);
        let later = dynamic_events(&block(vec![trade(4)]), &Events14::default(), |_| {
            Some(json!({"type":"TOKEN"}).to_string())
        });
        assert_eq!(later.events.len(), 1);
        assert!(
            dynamic_events(&block(vec![trade(4)]), &Events14::default(), |_| None)
                .events
                .is_empty()
        );
    }
    #[test]
    fn old_pump_and_forged_child_never_enter_v14() {
        let mut new = fixture(
            PUMP,
            PUMP_ABI,
            "NewToken",
            vec![Token::String("OLD".into()), addr(TOKEN), addr(USER)],
            1,
        );
        new.address = PUMP_V11.to_vec();
        assert!(static_events(&block(vec![new]), |_, _| None)
            .events
            .is_empty());
    }
    #[test]
    fn trade_curation_claims_discovered_in_same_transaction_and_future_blocks() {
        let new = fixture(
            FACTORY,
            FACTORY_ABI,
            "TradeCurationCreated",
            vec![addr(POOL), addr(ASSET), Token::String("Trade".into())],
            1,
        );
        let claim = fixture(
            POOL,
            CLAIM_ABI,
            "TradeClaimed",
            vec![addr(USER), uint(8), uint(123), Token::Bool(true)],
            2,
        );
        let blk = block(vec![new, claim.clone()]);
        let statics = static_events(&blk, |_, _| Some((TOKEN.into(), USER.into())));
        let events = dynamic_events(&blk, &statics, |_| None);
        let p = payload(&events.events[0]);
        assert_eq!(p["community"], ASSET);
        assert_eq!(p["asset"], TOKEN);
        let (_, meta) = discovery(&statics.events[0]).unwrap();
        assert_eq!(
            dynamic_events(&block(vec![claim]), &Events14::default(), |_| Some(
                meta.clone()
            ))
            .events
            .len(),
            1
        );
    }
    fn fee(i: u32) -> eth::Log {
        fixture(
            HOOK,
            HOOK_ABI,
            "SwapFeeCollected",
            vec![
                Token::FixedBytes(vec![1; 32]),
                addr(TOKEN),
                uint(2),
                uint(3),
                uint(5),
            ],
            i,
        )
    }
    fn swap(i: u32) -> eth::Log {
        let mut data = vec![0u8; 192];
        data[31] = 10;
        data[63] = 20;
        data[64..96].copy_from_slice(&ethabi::encode(&[Token::Uint(U256::from(1u64) << 96)]));
        eth::Log {
            address: CL_POOL_MANAGER.to_vec(),
            topics: vec![SWAP_TOPIC.to_vec(), vec![1; 32]],
            data,
            block_index: i,
            ordinal: i as u64 + 100,
            ..Default::default()
        }
    }
    #[test]
    fn v4_occurrence_matching_handles_both_fee_orders_and_ambiguous_receipts() {
        for logs in [
            vec![fee(1), swap(2)],
            vec![swap(1), fee(2)],
            vec![swap(1), fee(2), fee(3), swap(4)],
        ] {
            let es = static_events(&block(logs), |_, _| None);
            assert!(es.events.iter().all(|e| !payload(e)["swap"].is_null()));
        }
        let es = static_events(&block(vec![swap(1), fee(2), swap(3)]), |_, _| None);
        assert!(payload(&es.events[0])["swap"].is_null());
        let e = &es.events[0];
        let changes = event_changes(e, None, None);
        assert_eq!(
            field(&changes, "token_buyback_states", TOKEN, "native_reserve"),
            Some("5".into())
        );
        assert!(!changes
            .table_changes
            .iter()
            .any(|c| c.table == "token_trade_events"));
    }
    #[test]
    fn rh_curve_and_anti_snipe_supply_do_not_use_bsc_coefficient() {
        assert_eq!(rh_price(&BigInt::from(0)), BigInt::from(1_624_898_729u64));
        let last = rh_price(&parse_bigint("650000000000000000000000000"));
        assert!(last > BigInt::from(21_480_000_000u64) && last < BigInt::from(21_490_000_000u64));
        let injection = event(
            TOKEN,
            "AntiSnipeInjected",
            json!({"tokensPurchased":"10"}),
            1,
        );
        let sell = event(
            TOKEN,
            "Trade",
            json!({"isBuy":false,"tokenAmount":"100"}),
            2,
        );
        assert_eq!(
            supply_delta(&injection).unwrap() + supply_delta(&sell).unwrap(),
            BigInt::from(-90)
        );
        assert!(!event_changes(&injection, Some(BigInt::from(1010)), None)
            .table_changes
            .iter()
            .any(|c| c.table == "tokens"));
    }
    #[test]
    fn index_weights_and_optional_pool_reward_ratios_are_separate() {
        let config = event(
            PUMP,
            "IndexConfigured",
            json!({"token":TOKEN,"constituentAssets":[ASSET],"targetWeights":["10000"],"basketFeeBps":"30","creatorShareBps":"2000"}),
            1,
        );
        assert_eq!(
            field(
                &event_changes(&config, None, None),
                "token_components",
                &format!("{TOKEN}:{ASSET}"),
                "target_weight"
            ),
            Some("10000".into())
        );
        let link = event(
            PUMP,
            "NutboxStakingPoolLinked",
            json!({"token":TOKEN,"pool":POOL,"lpToken":USER,"rewardRatio":"2000"}),
            2,
        );
        let c = event_changes(&link, None, Some(format!("{TOKEN}:{ASSET}")));
        assert_eq!(
            field(
                &c,
                "token_components",
                &format!("{TOKEN}:{ASSET}"),
                "staking_reward_ratio"
            ),
            Some("2000".into())
        );
        assert_eq!(
            field(
                &c,
                "token_components",
                &format!("{TOKEN}:{ASSET}"),
                "target_weight"
            ),
            None
        );
        let optional = event(
            PUMP,
            "NutboxOptionalPoolLinked",
            json!({"token":TOKEN,"pool":POOL,"factory":USER,"rewardRatio":"8000"}),
            3,
        );
        assert_eq!(
            field(
                &event_changes(&optional, None, None),
                "token_optional_pools",
                &format!("{TOKEN}:{POOL}"),
                "factory"
            ),
            Some(USER.into())
        );
    }
    #[test]
    fn recovery_and_queued_events_keep_chain_order() {
        let queued = event(TOKEN, "TokenListingQueued", json!({"token":TOKEN}), 3);
        let recovered = event(PUMP, "FailedListingRecovered", json!({"token":TOKEN}), 4);
        let es = normalize(vec![recovered, queued]);
        let outputs: Vec<_> = es.iter().map(|e| event_changes(e, None, None)).collect();
        assert_eq!(
            field(&outputs[0], "tokens", TOKEN, "listing_status"),
            Some("LISTING_PENDING".into())
        );
        assert_eq!(
            field(&outputs[1], "tokens", TOKEN, "listing_status"),
            Some("BONDING_CURVE".into())
        );
        assert!(outputs[0].table_changes.iter().all(|c| c.ordinal == 103));
        assert!(outputs[1].table_changes.iter().all(|c| c.ordinal == 104));
    }
    #[test]
    fn buyback_deducts_only_spent_native_and_retains_nested_fees() {
        let fee = event(
            HOOK,
            "SwapFeeCollected",
            json!({"token":TOKEN,"buybackFee":"10","platformFee":"0","deployerFee":"0"}),
            1,
        );
        let buy = event(
            HOOK,
            "BuybackExecuted",
            json!({"token":TOKEN,"indexToken":ASSET,"bnbIn":"7","indexOut":"123"}),
            2,
        );
        assert_eq!(
            field(
                &event_changes(&fee, None, None),
                "token_buyback_states",
                TOKEN,
                "native_reserve"
            ),
            Some("10".into())
        );
        assert_eq!(
            field(
                &event_changes(&buy, None, None),
                "token_buyback_states",
                TOKEN,
                "native_reserve"
            ),
            Some("-7".into())
        );
    }
    #[test]
    fn trade_claim_uses_log_id_not_order_id_and_never_creates_stake_or_social_claim() {
        for (i, user, harvested) in [(1, USER, true), (2, ASSET, false)] {
            let e = event(
                POOL,
                "TradeClaimed",
                json!({"user":user,"community":TOKEN,"asset":ASSET,"orderId":"42","amount":"100","harvested":harvested}),
                i,
            );
            let c = event_changes(&e, None, None);
            let key = event_id(&e.transaction_hash, i);
            assert_eq!(
                field(&c, "walnut_operations", &key, "trade_order_id"),
                Some("42".into())
            );
            assert_eq!(
                field(&c, "walnut_operations", &key, "social_order_id"),
                None
            );
            assert_eq!(
                field(&c, "walnut_pools", POOL, "total_claimed"),
                Some("100".into())
            );
            assert_eq!(field(&c, "walnut_pools", POOL, "total_amount"), None);
            assert!(!c
                .table_changes
                .iter()
                .any(|c| c.table == "walnut_pool_stakers"));
        }
    }
    #[test]
    fn router_state_is_scoped_to_deployed_v14_router() {
        let e = event(
            ROUTER,
            "PricePoolAdded",
            json!({"poolId":"0x01","token0":TOKEN,"token1":ASSET,"sourceType":"4","sourceData":"0xab"}),
            1,
        );
        let c = event_changes(&e, None, None);
        assert_eq!(
            field(
                &c,
                "v14_router_price_pools",
                &format!("{ROUTER}:0x01"),
                "source_type"
            ),
            Some("4".into())
        );
        assert!(!c
            .table_changes
            .iter()
            .any(|c| c.table == "nutbox_router_price_pools"));
    }
    #[test]
    fn frozen_successful_deployment_receipts_decode_all_initial_stocks_and_routes() {
        let receipts: Value =
            serde_json::from_str(include_str!("../fixtures/rh-v14-deployment-logs.json")).unwrap();
        let contracts: BTreeMap<_, _> = [
            (PUMP, abi(PUMP_ABI)),
            (ROUTER, abi(ROUTER_ABI)),
            (FACTORY, abi(FACTORY_ABI)),
            (TRADE_ROUTER, abi(TRADE_ABI)),
        ]
        .into_iter()
        .collect();
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for receipt in receipts.as_array().unwrap() {
            for l in receipt["logs"].as_array().unwrap() {
                let Some(a) = contracts.get(l["address"].as_str().unwrap().to_lowercase().as_str())
                else {
                    continue;
                };
                let log = eth::Log {
                    address: bytes(l["address"].as_str().unwrap()),
                    data: bytes(l["data"].as_str().unwrap()),
                    topics: l["topics"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| bytes(v.as_str().unwrap()))
                        .collect(),
                    ..Default::default()
                };
                if let Some((name, _)) = decode(a, &log) {
                    *counts.entry(name).or_default() += 1;
                }
            }
        }
        assert_eq!(counts["ConstituentApprovalSet"], 52);
        assert_eq!(counts["PricePoolAdded"], 53);
        assert_eq!(counts["RouteAdded"], 105);
        assert_eq!(counts["OptionalPoolFactorySet"], 1);
    }
    #[test]
    fn v14_manifest_is_address_scoped_and_stores_start_at_real_deployments() {
        let manifest = include_str!("../substreams.yaml");
        for a in [PUMP, HOOK, ROUTER, FACTORY, TRADE_ROUTER, LIQUIDITY_ROUTER] {
            assert!(manifest.contains(a));
        }
        assert!(manifest.contains("initialBlock: 83024792"));
        assert!(manifest.contains("initialBlock: 83065677"));
        assert!(!manifest.contains(&format!(
            "evt_sig:0x{}",
            hex::encode(abi(TOKEN_ABI).event("Transfer").unwrap().signature())
        )));
        assert!(!manifest.contains(&format!("evt_sig:0x{}", hex::encode(SWAP_TOPIC))));
        assert!(include_str!("../examples/make_v14_continuation.rs")
            .contains("production.sink_module.clone()"));
    }
    #[test]
    fn basket_v14_reuses_trade_semantics_without_replaying_registry_or_token_fees() {
        let hook = prefixed_hex(&BASKET_HOOK);
        let a = include_str!("../abi/basket_hook.abi.json");
        let ev = abi(a).event("BasketBought").unwrap().clone();
        let args = ev.inputs.iter().map(|p| default_token(&p.kind)).collect();
        let blk = block(vec![fixture(&hook, a, "BasketBought", args, 5)]);
        let es = map_basket_events_for(
            &blk,
            &contract::BasketRegistryEvents::default(),
            &StoreGetInt64::new(0),
            BasketEventSelection {
                hooks: &[BASKET_HOOK],
                routers: &[BASKET_ROUTER],
                include_fee_auction: false,
                include_v1_rebalances: false,
                include_v3_rebalances: false,
                include_token_events: false,
            },
        );
        assert_eq!(es.trades.len(), 1);
        assert!(es.fee_accruals.is_empty());
        assert!(es.fee_claims.is_empty());
        assert!(es.auction_events.is_empty());
        let changes = basket_changes(es);
        assert!(changes.table_changes.iter().all(|c| c.ordinal == 105));
        assert_eq!(
            field(&changes, "baskets", TOKEN, "buy_count"),
            Some("1".into())
        );
        assert_eq!(field(&changes, "baskets", TOKEN, "creator"), None);
    }
    #[test]
    fn same_block_close_is_not_overwritten_by_trade_factory_metadata() {
        let e = event(
            FACTORY,
            "TradeCurationCreated",
            json!({"pool":POOL,"community":ASSET,"asset":TOKEN,"owner":USER,"name":"Trade","closedInBlock":true}),
            1,
        );
        let changes = event_changes(&e, None, None);
        assert_eq!(
            field(&changes, "walnut_pools", POOL, "status"),
            Some("CLOSED".into())
        );
        assert_eq!(field(&changes, "walnut_pools", POOL, "ratio"), None);
        assert_eq!(
            field(&changes, "walnut_pools", POOL, "total_amount"),
            Some("0".into())
        );
    }
    #[test]
    fn component_tax_and_reward_notifications_materialize_correct_assets() {
        let key = format!("{TOKEN}:{ASSET}");
        let e = event(
            TOKEN,
            "ComponentPoolTaxBurned",
            json!({"pair":POOL,"from":USER,"to":ASSET,"grossAmount":"1000","taxAmount":"10"}),
            1,
        );
        assert_eq!(
            field(
                &event_changes(&e, None, Some(key.clone())),
                "token_components",
                &key,
                "total_tax_to_dead"
            ),
            Some("10".into())
        );
        let e = event(
            TOKEN,
            "BuybackRewardNotified",
            json!({"indexToken":ASSET,"amount":"99","accRewardPerToken":"12345"}),
            2,
        );
        let changes = event_changes(&e, None, None);
        assert_eq!(
            field(&changes, "token_buyback_states", TOKEN, "index_token"),
            Some(ASSET.into())
        );
        assert_eq!(
            field(
                &changes,
                "token_buyback_states",
                TOKEN,
                "total_index_notified"
            ),
            Some("99".into())
        );
    }
    #[test]
    fn complete_token_lifecycle_has_valid_supply_fees_and_listing_fields() {
        let buy = event(
            TOKEN,
            "Trade",
            json!({"buyer":USER,"sellsman":ASSET,"isBuy":true,"tokenAmount":"1000","ethAmount":"300","tiptagFee":"3","sellsmanFee":"6"}),
            2,
        );
        let changes = event_changes(&buy, Some(BigInt::from(1000)), None);
        assert_eq!(
            field(&changes, "tokens", TOKEN, "bonding_curve_supply"),
            Some("1000".into())
        );
        assert_eq!(
            field(&changes, "tokens", TOKEN, "tiptag_fee"),
            Some("3".into())
        );
        assert_eq!(
            field(
                &changes,
                "token_trade_events",
                &event_id(&buy.transaction_hash, 2),
                "venue"
            ),
            Some("BONDING_CURVE".into())
        );
        let listed = event(
            TOKEN,
            "TokenListedToDex",
            json!({"token":TOKEN,"poolId":"0x01","sqrtPriceX96":"501082896750095862372827603139212"}),
            3,
        );
        let c = event_changes(&listed, None, None);
        assert_eq!(
            field(&c, "tokens", TOKEN, "listing_status"),
            Some("LISTED".into())
        );
        assert_eq!(
            field(&c, "tokens", TOKEN, "listing_queued_at"),
            Some("0".into())
        );
        assert_eq!(
            field(&c, "pairs", "0x01", "venue"),
            Some("UNISWAP_V4".into())
        );
        let sell = event(
            TOKEN,
            "Trade",
            json!({"buyer":USER,"sellsman":ASSET,"isBuy":false,"tokenAmount":"100","ethAmount":"30","tiptagFee":"1","sellsmanFee":"2"}),
            4,
        );
        assert_eq!(
            field(
                &event_changes(&sell, Some(BigInt::from(900)), None),
                "tokens",
                TOKEN,
                "sell_times"
            ),
            Some("1".into())
        );
    }
    #[test]
    fn v14_schema_keeps_read_views_outside_sink_public_schema() {
        let sql = include_str!("../scripts/migrate-rh-v14-schema.sql");
        assert!(sql.contains("BEGIN;"));
        assert!(sql.contains("COMMIT;"));
        assert!(sql.contains("CREATE OR REPLACE VIEW rh_read.walnut_account_communities"));
        assert!(sql.contains("ON walnut_pools (entity_index) WHERE entity_index > 0"));
        assert!(sql.contains("ALTER TABLE pairs ALTER COLUMN token_index SET DEFAULT 0"));
    }
}
