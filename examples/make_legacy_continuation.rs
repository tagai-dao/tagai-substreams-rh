#![allow(dead_code)]

#[path = "../src/pb/mod.rs"]
mod pb;

use pb::sf::substreams::v1::{module::block_filter::Query, Package};
use prost::Message;
use std::{env, fs, path::Path};

const DISABLED_BASKET_FILTER: &str = "evt_addr:0x0000000000000000000000000000000000000000 && evt_addr:0xffffffffffffffffffffffffffffffffffffffff";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let input = args
        .next()
        .ok_or("usage: make_legacy_continuation <input.spkg> <output.spkg>")?;
    let output = args
        .next()
        .ok_or("usage: make_legacy_continuation <input.spkg> <output.spkg>")?;
    if args.next().is_some() {
        return Err("usage: make_legacy_continuation <input.spkg> <output.spkg>".into());
    }

    let input_path = Path::new(&input);
    let output_path = Path::new(&output);
    if input_path == output_path {
        return Err("input and output paths must differ".into());
    }

    let bytes = fs::read(input_path)?;
    let mut package = Package::decode(bytes.as_slice())?;
    let modules = package
        .modules
        .as_mut()
        .ok_or("SPKG does not contain a modules section")?;

    let mut changed = Vec::new();
    for module in &mut modules.modules {
        if !matches!(
            module.name.as_str(),
            "map_basket_registry_events" | "map_basket_events"
        ) {
            continue;
        }

        let block_filter = module
            .block_filter
            .as_mut()
            .ok_or_else(|| format!("{} has no block filter", module.name))?;
        block_filter.query = Some(Query::QueryString(DISABLED_BASKET_FILTER.to_string()));
        changed.push(module.name.clone());
    }

    if changed.len() != 2 {
        return Err(format!(
            "expected two Basket map modules, changed {} ({})",
            changed.len(),
            changed.join(", ")
        )
        .into());
    }

    for module_name in &changed {
        if package.block_filters.contains_key(module_name) {
            package
                .block_filters
                .insert(module_name.clone(), DISABLED_BASKET_FILTER.to_string());
        }
    }

    let mut encoded = Vec::with_capacity(package.encoded_len());
    package.encode(&mut encoded)?;
    fs::write(output_path, encoded)?;

    println!("created {}", output_path.display());
    println!("disabled filters: {}", changed.join(", "));
    println!("all WASM binaries and non-Basket modules were preserved byte-for-byte");
    Ok(())
}
