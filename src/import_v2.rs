//! New address-scoped, stateless domain. Backfill writes only independent tables.
//! Legacy maps/stores/WASM stay byte-for-byte from the installed SPKG.
use super::*;
use super::v14::{abi, decode, envelope, normalize, payload, string, number, flag};
const WRAPPER: &str = "0x53e65de68a0eb7f3662579f44eb9849ae5ca44ab";
const OLD_HELPER: &str = "0xd0baaf8d314b0c6ab6bebd24ddbc86157c9c7b09";
const HELPER: &str = "0x70ed7c79d80dede1828a09b3c526df283111cb82";
const SOCIAL: &str = "0xddbaba530728b5b8939d7fddc334432490916e90";
const FIRST_BLOCK: u64 = 54_128_714;
const HELPER_BLOCK: u64 = 86_448_655;
const HELPER_ABI: &str = include_str!("../abi/import-v2/ImportHelperV2.json");
const WRAPPER_ABI: &str = include_str!("../abi/import-v2/ImportedTokenSwapWrapper.json");

fn events(block: &eth::Block) -> contract::V14Events {
    let helper = abi(HELPER_ABI);
    let wrapper = abi(WRAPPER_ABI);
    let mut output = Vec::new();
    if block.number < FIRST_BLOCK { return contract::V14Events::default(); }
    for receipt in block.receipts() {
        for log in &receipt.receipt.logs {
            let address = prefixed_hex(&log.address);
            let source = match address.as_str() {
                WRAPPER => &wrapper,
                OLD_HELPER if block.number >= 54_128_745 => &helper,
                HELPER if block.number >= HELPER_BLOCK => &helper,
                _ => continue,
            };
            if let Some((kind, p)) = decode(source, log) {
                output.push(envelope(block, receipt.transaction, log, kind, p));
            }
        }
    }
    contract::V14Events { events: normalize(output) }
}

fn write(events: contract::V14Events) -> DatabaseChanges {
    let mut tables = Tables::new();
    let mut ordinals = std::collections::BTreeMap::new();
    for e in events.events {
        let p = payload(&e);
        let id = format!("{}-{}", e.transaction_hash, e.log_index);
        ordinals.insert(("import_v2_events".to_string(),id.clone()),e.ordinal);
        tables.upsert_row("import_v2_events", &id)
            .set("kind", &e.kind).set("source", &e.source).set("payload", &e.payload)
            .set("block_number", e.block_number).set("block_hash", &e.block_hash)
            .set("block_timestamp", e.timestamp).set("transaction_hash", &e.transaction_hash)
            .set("log_index", e.log_index);
        match e.kind.as_str() {
            "CommunityCreated" | "ImportedCommunityRecorded" | "ImportedMarketRegistered" | "ImportedMarketUpdated" | "ImportedPoolCreated" => {
                let token = string(&p, "token");
                ordinals.insert(("import_v2_markets".to_string(),token.to_string()),e.ordinal);
                let row = tables.upsert_row("import_v2_markets", token);
                row.set("source", &e.source).set("updated_block", e.block_number)
                    .set("updated_at", e.timestamp).set("updated_log_index", e.log_index);
                if e.kind == "ImportedMarketUpdated" {
                    row.set("community", string(&p, "newCommunity")).set("deployer", string(&p, "newDeployer"));
                } else {
                    row.set("community", string(&p, "community"));
                    if e.kind == "ImportedMarketRegistered" { row.set("deployer", string(&p, "deployer")); }
                    if matches!(e.kind.as_str(), "CommunityCreated" | "ImportedCommunityRecorded") { row.set("deployer", string(&p, "creator")); }
                    if e.kind == "CommunityCreated" {
                        row.set("calculator", string(&p, "calculator"));
                        // V2's first pool may be TradeCuration. Never mislabel it SocialCuration.
                        if e.source == OLD_HELPER { row.set("social_pool", string(&p, "pool")); }
                    }
                    if e.kind == "ImportedPoolCreated" && string(&p, "factory") == SOCIAL {
                        row.set("social_pool", string(&p, "pool"));
                    }
                }
            }
            "ImportedTokenTrade" if e.source == WRAPPER => {
                let source: usize = string(&p, "sourceType").parse().expect("source type");
                let venue = ["V2", "V3", "V4", "V4"]; // Native/wrapped and stable quote legs share the actual wrapper event.
                let venue = venue.get(source).expect("unsupported wrapper source");
                ordinals.insert(("import_v2_trade_events".to_string(),id.clone()),e.ordinal);
                tables.upsert_row("import_v2_trade_events", id)
                    .set("entity_index", (e.block_number * 1_000_000 + e.log_index as u64) as i64)
                    .set("venue", *venue).set("token", string(&p, "token"))
                    .set("trader", string(&p, "trader")).set("sellsman", string(&p, "sellsman"))
                    .set("recipient", string(&p, "recipient")).set("is_buy", flag(&p, "isBuy"))
                    .set("token_amount", number(&p, "tokenAmount"))
                    .set("native_amount", number(&p, "grossNativeAmount"))
                    .set("nutbox_token_fee", number(&p, "nutboxTokenFee"))
                    .set("block_number", e.block_number).set("block_hash", &e.block_hash)
                    .set("block_timestamp", e.timestamp).set("transaction_hash", &e.transaction_hash)
                    // Dedicated domain's position is the canonical global log index, not a call trace index.
                    .set("call_index", e.log_index);
            }
            _ => {},
        }
    }
    let mut changes = tables.to_database_changes();
    for change in &mut changes.table_changes {
        let key = match &change.primary_key {
            Some(substreams_database_change::pb::sf::substreams::sink::database::v1::table_change::PrimaryKey::Pk(key)) => key,
            _ => panic!("reviewed single-column import primary key"),
        };
        change.ordinal = ordinals[&(change.table.clone(),key.clone())];
        change.fields.sort_by(|a,b|a.name.cmp(&b.name));
    }
    changes.table_changes.sort_by(|a,b|a.ordinal.cmp(&b.ordinal).then_with(||a.table.cmp(&b.table)).then_with(||format!("{:?}",a.primary_key).cmp(&format!("{:?}",b.primary_key))));
    changes
}
#[substreams::handlers::map]
fn map_import_v2_events(block: eth::Block) -> Result<contract::V14Events, substreams::errors::Error> { Ok(events(&block)) }
#[substreams::handlers::map]
fn import_v2_backfill_db_out(events: contract::V14Events) -> Result<DatabaseChanges, substreams::errors::Error> { Ok(write(events)) }
#[substreams::handlers::map]
fn import_v2_continuation_db_out(old: DatabaseChanges, imports: DatabaseChanges) -> Result<DatabaseChanges, substreams::errors::Error> { Ok(merge_database_changes([old, imports])) }

#[cfg(test)]
mod tests {
    use super::*;
    use ethabi::{Token, ethereum_types::{H160, U256}};
    use serde_json::json;
    const TOKEN: &str = "0x1111111111111111111111111111111111111111";
    const USER: &str = "0x2222222222222222222222222222222222222222";
    const POOL: &str = "0x3333333333333333333333333333333333333333";
    fn addr(a: &str) -> Token { Token::Address(H160::from_slice(&hex::decode(&a[2..]).unwrap())) }
    fn uint(n: u64) -> Token { Token::Uint(U256::from(n)) }
    fn log(source: &str, abi_str: &str, kind: &str, args: Vec<Token>, index: u32) -> eth::Log {
        let contract = abi(abi_str); let event = contract.event(kind).unwrap();
        assert_eq!(args.len(), event.inputs.len());
        let mut topics = vec![event.signature().as_bytes().to_vec()]; let mut data = vec![];
        for (param, value) in event.inputs.iter().zip(args) {
            if param.indexed { topics.push(ethabi::encode(&[value])); } else { data.push(value); }
        }
        eth::Log { address: hex::decode(&source[2..]).unwrap(), topics, data: ethabi::encode(&data), block_index:index, ordinal:100+index as u64,..Default::default() }
    }
    fn block(logs: Vec<eth::Log>, number: u64) -> eth::Block {
        eth::Block { number, hash:vec![7;32], header:Some(eth::BlockHeader {timestamp:Some(prost_types::Timestamp{seconds:1_791_780_000,nanos:0}),..Default::default()}),transaction_traces:vec![eth::TransactionTrace{from:hex::decode(&USER[2..]).unwrap(),hash:vec![9;32],status:eth::TransactionTraceStatus::Succeeded as i32,receipt:Some(eth::TransactionReceipt{logs,..Default::default()}),..Default::default()}],..Default::default() }
    }
    fn values(changes: &DatabaseChanges, table: &str) -> serde_json::Value {
        let c=changes.table_changes.iter().find(|c| c.table==table).unwrap();
        c.fields.iter().map(|f|(f.name.clone(),json!(f.value))).collect()
    }
    #[test]
    fn all_venues_buys_sells_use_actual_wrapper_amounts_and_only_isolated_tables() {
        for source in 0..4 { for buy in [true,false] {
            let trade=log(WRAPPER,WRAPPER_ABI,"ImportedTokenTrade",vec![addr(USER),addr(TOKEN),addr(USER),Token::Bool(buy),addr(POOL),uint(source),Token::FixedBytes(vec![4;32]),uint(97),uint(5),uint(4),uint(1),uint(0),uint(3),addr(USER)],5);
            let changes=write(events(&block(vec![trade],HELPER_BLOCK)));
            assert_eq!(changes.table_changes.len(),2);
            assert!(changes.table_changes.iter().all(|c|c.table.starts_with("import_v2_")));
            let p=values(&changes,"import_v2_trade_events");
            assert_eq!(p["token_amount"],"97"); assert_eq!(p["native_amount"],"5");
            assert_eq!(p["nutbox_token_fee"],"3"); assert_eq!(p["call_index"],"5");
        }}
    }
    #[test]
    fn same_transaction_pool_links_preserve_social_pool_but_never_label_first_trade_pool_social() {
        let create=log(HELPER,HELPER_ABI,"CommunityCreated",vec![addr(TOKEN),addr(POOL),addr(USER),addr(USER),addr(USER)],5);
        let link=log(HELPER,HELPER_ABI,"ImportedPoolCreated",vec![addr(TOKEN),addr(POOL),addr(POOL),addr(SOCIAL),uint(3000),uint(1)],2);
        let p=values(&write(events(&block(vec![create.clone(),link],HELPER_BLOCK))),"import_v2_markets");
        assert_eq!(p["social_pool"],POOL);assert_eq!(p["calculator"],USER);
        let p=values(&write(events(&block(vec![create],HELPER_BLOCK))),"import_v2_markets");
        assert!(p.get("social_pool").is_none());
    }
    #[test]
    fn exact_addresses_boundaries_duplicates_and_replays_are_deterministic() {
        let l=log(WRAPPER,WRAPPER_ABI,"RegistrarChanged",vec![addr(OLD_HELPER),addr(HELPER)],3);
        assert!(events(&block(vec![l.clone()],FIRST_BLOCK-1)).events.is_empty());
        let mut foreign=l.clone();foreign.address=hex::decode(&TOKEN[2..]).unwrap();
        let b=block(vec![l.clone(),l,foreign],FIRST_BLOCK);
        assert_eq!(events(&b).events.len(),1);
        assert_eq!(write(events(&b)),write(events(&b)));
        let l=log(HELPER,HELPER_ABI,"ImportedCommunityRecorded",vec![addr(TOKEN),addr(POOL),addr(USER)],1);
        assert!(events(&block(vec![l],HELPER_BLOCK-1)).events.is_empty());
    }
    #[test]
    fn failed_transaction_and_malformed_known_events_do_not_produce_silent_trades() {
        let l=log(WRAPPER,WRAPPER_ABI,"RegistrarChanged",vec![addr(OLD_HELPER),addr(HELPER)],3);
        let mut b=block(vec![l.clone()],FIRST_BLOCK);b.transaction_traces[0].status=eth::TransactionTraceStatus::Failed as i32;
        assert!(events(&b).events.is_empty());
        let mut malformed=l;malformed.topics.pop();
        assert!(std::panic::catch_unwind(||events(&block(vec![malformed],FIRST_BLOCK))).is_err());
    }
    #[test]
    fn manifest_new_backfill_is_store_free_exact_scoped_and_separate_from_old_output() {
        let manifest=include_str!("../substreams.yaml");
        let section=manifest.split("  - name: map_import_v2_events").nth(1).unwrap();
        assert!(section.contains("initialBlock: 54128714"));
        assert!(section.contains(WRAPPER)&&section.contains(OLD_HELPER)&&section.contains(HELPER));
        assert!(!section.contains("evt_sig:")&&!section.contains("store:"));
        let backfill=section.split("  - name: import_v2_backfill_db_out").nth(1).unwrap().split("  - name: import_v2_continuation_db_out").next().unwrap();
        assert!(!backfill.contains("v14_continuation_db_out"));
    }
    #[test]
    fn real_public_deployment_receipts_decode_initial_wrapper_and_v2_configuration() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!("../fixtures/rh-import-v2-deployment-logs.json")).unwrap();
        for fixture in fixtures.as_array().unwrap() {
            let logs=fixture["logs"].as_array().unwrap().iter().map(|l|eth::Log {
                address:hex::decode(l["address"].as_str().unwrap().trim_start_matches("0x")).unwrap(),
                topics:l["topics"].as_array().unwrap().iter().map(|t|hex::decode(t.as_str().unwrap().trim_start_matches("0x")).unwrap()).collect(),
                data:hex::decode(l["data"].as_str().unwrap().trim_start_matches("0x")).unwrap(),
                block_index:l["logIndex"].as_u64().unwrap() as u32,
                ordinal:l["logIndex"].as_u64().unwrap()+100,
                ..Default::default()
            }).collect::<Vec<_>>();
            let count=logs.len();
            let output=events(&block(logs,fixture["block"].as_u64().unwrap()));
            assert_eq!(output.events.len(),count);
            let changes=write(output);
            assert!(changes.table_changes.iter().all(|c|c.table.starts_with("import_v2_")));
        }
    }

    #[test]
    fn every_reviewed_helper_and_wrapper_event_family_preserves_named_payload_and_ordinal() {
        fn default(p:&ethabi::ParamType)->Token {
            match p {
                ethabi::ParamType::Address=>addr(TOKEN),
                ethabi::ParamType::Uint(_)|ethabi::ParamType::Int(_)=>uint(1),
                ethabi::ParamType::Bool=>Token::Bool(true),
                ethabi::ParamType::String=>Token::String("fixture".into()),
                ethabi::ParamType::Bytes=>Token::Bytes(vec![1]),
                ethabi::ParamType::FixedBytes(n)=>Token::FixedBytes(vec![1;*n]),
                ethabi::ParamType::Array(p)=>Token::Array(vec![default(p)]),
                ethabi::ParamType::FixedArray(p,n)=>Token::FixedArray((0..*n).map(|_|default(p)).collect()),
                ethabi::ParamType::Tuple(ps)=>Token::Tuple(ps.iter().map(default).collect()),
            }
        }
        for (source, schema) in [(HELPER,HELPER_ABI),(WRAPPER,WRAPPER_ABI)] {
            for event in abi(schema).events() {
                let l=log(source,schema,&event.name,event.inputs.iter().map(|p|default(&p.kind)).collect(),7);
                let parsed=events(&block(vec![l],HELPER_BLOCK));
                assert_eq!(parsed.events.len(),1);assert_eq!(parsed.events[0].kind,event.name);
                let data=payload(&parsed.events[0]);
                for param in &event.inputs {assert!(data.get(&param.name).is_some());}
                let changes=write(parsed);
                assert!(changes.table_changes.iter().all(|c|c.ordinal==107));
            }
        }
    }

}
