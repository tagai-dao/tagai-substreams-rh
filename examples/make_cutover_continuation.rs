#![allow(dead_code)]

#[path = "../src/pb/mod.rs"]
mod pb;

use pb::sf::substreams::v1::Package;
use prost::Message;
use std::{collections::HashSet, env, fs, path::Path};

const FINAL_OUTPUT_MODULE: &str = "v11_continuation_db_out";

// V0.5.2 changed the block filters in the imported-trade and IndexBroker
// branches. Their pre-cutover state was verified empty (apart from immutable
// IndexBroker factory/event rows already written to PostgreSQL), so they start
// fresh at the cutover boundary instead of preparing their old module graphs.
//
// The V11 TagAI and Basket branches are deliberately absent: they carry
// stateful token-address, supply, and dynamic-basket stores that must continue
// from the completed additive backfill cache.
const CUTOVER_RESET_MODULES: &[&str] = &[
    "map_v11_protocol_events",
    "v11_protocol_db_out",
    "store_imported_market_deployers",
    "map_imported_trade_events",
    "store_imported_trade_indexes",
    "imported_trade_db_out",
    "map_index_broker_factory_events",
    "store_index_broker_pools",
    "store_index_broker_amms",
    "map_index_broker_pool_events",
    "map_index_broker_amm_events",
    "store_index_broker_indexes",
    "store_index_broker_accounts",
    "index_broker_db_out",
    FINAL_OUTPUT_MODULE,
];

fn usage() -> &'static str {
    "usage: make_cutover_continuation <input.spkg> <cutover-start-block> <output.spkg>"
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let input_path = args.next().ok_or_else(usage)?;
    let cutover_start = args
        .next()
        .ok_or_else(usage)?
        .into_string()
        .map_err(|_| "cutover start block must be valid UTF-8")?
        .parse::<u64>()?;
    let output_path = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage().into());
    }
    if cutover_start == 0 {
        return Err("cutover start block must be positive".into());
    }
    if Path::new(&input_path) == Path::new(&output_path) {
        return Err("output path must differ from input path".into());
    }

    let mut package = Package::decode(fs::read(&input_path)?.as_slice())?;
    if package.sink_module != FINAL_OUTPUT_MODULE {
        return Err(format!(
            "input package sink must be {FINAL_OUTPUT_MODULE:?}, got {:?}",
            package.sink_module
        )
        .into());
    }

    let modules = package
        .modules
        .as_mut()
        .ok_or("SPKG does not contain a modules section")?;
    let required = CUTOVER_RESET_MODULES
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let present = modules
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect::<HashSet<_>>();
    let mut missing = required.difference(&present).copied().collect::<Vec<_>>();
    missing.sort_unstable();
    if !missing.is_empty() {
        return Err(format!("input package is missing cutover modules: {missing:?}").into());
    }

    for module in &mut modules.modules {
        if required.contains(module.name.as_str()) {
            if module.initial_block > cutover_start {
                return Err(format!(
                    "module {:?} starts at {} after requested cutover {}",
                    module.name, module.initial_block, cutover_start
                )
                .into());
            }
            module.initial_block = cutover_start;
        }
    }

    // Keep existing network-specific overrides aligned. Do not add overrides
    // where the source package intentionally had none.
    for network in package.networks.values_mut() {
        for name in CUTOVER_RESET_MODULES {
            if let Some(initial_block) = network.initial_blocks.get_mut(*name) {
                *initial_block = cutover_start;
            }
        }
    }

    let mut encoded = Vec::with_capacity(package.encoded_len());
    package.encode(&mut encoded)?;
    fs::write(&output_path, encoded)?;

    println!("created {}", Path::new(&output_path).display());
    println!("cutover start block: {cutover_start}");
    println!("retargeted modules: {}", CUTOVER_RESET_MODULES.len());
    println!("sink module: {FINAL_OUTPUT_MODULE}");
    println!("required database precondition: no imported markets/trades or IndexBroker pools, AMMs, tokens, or accounts before cutover");
    Ok(())
}
