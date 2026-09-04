#![allow(dead_code)]

#[path = "../src/pb/mod.rs"]
mod pb;

use pb::sf::substreams::v1::{Binary, Module, Modules, Package};
use prost::Message;
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::Path,
};

const FINAL_OUTPUT_MODULE: &str = "v11_continuation_db_out";
const BACKFILL_OUTPUT_MODULE: &str = "v11_backfill_db_out";

fn usage() -> &'static str {
    "usage: make_additive_continuation \
<exact-production.spkg> <additive-template.spkg> <output.spkg>"
}

fn modules(package: &Package) -> Result<&Modules, Box<dyn std::error::Error>> {
    package
        .modules
        .as_ref()
        .ok_or_else(|| "SPKG does not contain a modules section".into())
}

fn module_map(package: &Package) -> Result<HashMap<&str, &Module>, Box<dyn std::error::Error>> {
    Ok(modules(package)?
        .modules
        .iter()
        .map(|module| (module.name.as_str(), module))
        .collect())
}

fn append_binary(
    source: &Modules,
    source_index: u32,
    destination: &mut Vec<Binary>,
    remapped_indexes: &mut HashMap<u32, u32>,
) -> Result<u32, Box<dyn std::error::Error>> {
    if let Some(index) = remapped_indexes.get(&source_index) {
        return Ok(*index);
    }

    let binary = source
        .binaries
        .get(source_index as usize)
        .ok_or_else(|| format!("binary index {source_index} is out of range"))?
        .clone();
    let destination_index = destination.len() as u32;
    destination.push(binary);
    remapped_indexes.insert(source_index, destination_index);
    Ok(destination_index)
}

fn copy_module(
    source_module: &Module,
    source_modules: &Modules,
    destination_binaries: &mut Vec<Binary>,
    remapped_indexes: &mut HashMap<u32, u32>,
) -> Result<Module, Box<dyn std::error::Error>> {
    let mut copied = source_module.clone();
    copied.binary_index = append_binary(
        source_modules,
        source_module.binary_index,
        destination_binaries,
        remapped_indexes,
    )?;
    Ok(copied)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let production_path = args.next().ok_or_else(usage)?;
    let template_path = args.next().ok_or_else(usage)?;
    let output_path = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage().into());
    }

    let output_path = Path::new(&output_path);
    if Path::new(&production_path) == output_path || Path::new(&template_path) == output_path {
        return Err("output path must differ from both input paths".into());
    }

    let production = Package::decode(fs::read(&production_path)?.as_slice())?;
    let mut continuation = Package::decode(fs::read(&template_path)?.as_slice())?;
    if production.network != continuation.network {
        return Err(format!(
            "network mismatch: production={:?}, template={:?}",
            production.network, continuation.network
        )
        .into());
    }

    let production_modules = modules(&production)?;
    let production_by_name = module_map(&production)?;
    let production_names = production_by_name.keys().copied().collect::<HashSet<_>>();
    let template_names = modules(&continuation)?
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect::<HashSet<_>>();

    for required in &production_names {
        if !template_names.contains(required) {
            return Err(format!(
                "additive template is missing production module {required:?}; \
                 removing an old module is not a continuation"
            )
            .into());
        }
    }
    for required in [FINAL_OUTPUT_MODULE, BACKFILL_OUTPUT_MODULE] {
        if production_names.contains(required) {
            return Err(format!(
                "new output module {required:?} already exists in production; \
                 use a release-specific output name"
            )
            .into());
        }
        if !template_names.contains(required) {
            return Err(format!("additive template is missing output module {required:?}").into());
        }
    }

    // The new package starts with all template modules, but every name shared
    // with production is replaced by the exact production definition and
    // binary. New contracts must therefore use unique module names.
    let continuation_modules = continuation
        .modules
        .as_mut()
        .ok_or("additive template does not contain a modules section")?;
    let mut destination_binaries = continuation_modules.binaries.clone();
    let mut production_binary_indexes = HashMap::new();
    let mut preserved = Vec::new();
    let mut added = Vec::new();

    for destination_module in &mut continuation_modules.modules {
        if let Some(source_module) = production_by_name.get(destination_module.name.as_str()) {
            let name = destination_module.name.clone();
            *destination_module = copy_module(
                source_module,
                production_modules,
                &mut destination_binaries,
                &mut production_binary_indexes,
            )?;
            preserved.push(name);
        } else {
            added.push(destination_module.name.clone());
        }
    }
    continuation_modules.binaries = destination_binaries;

    // Block-filter metadata lives both on Module and in this display map.
    // The executable definition was copied above; keep the human-readable map
    // aligned with the exact production source as well.
    for name in &preserved {
        match production.block_filters.get(name) {
            Some(filter) => {
                continuation
                    .block_filters
                    .insert(name.clone(), filter.clone());
            }
            None => {
                continuation.block_filters.remove(name);
            }
        }
    }

    // Network-specific initial blocks and module parameters are stored outside
    // Module. Remove every template override for preserved modules first, then
    // restore the exact values (including intentional absence) from production.
    // Template-only modules keep their additive network configuration.
    for network_params in continuation.networks.values_mut() {
        for name in &preserved {
            network_params.initial_blocks.remove(name);
            network_params.params.remove(name);
        }
    }
    for (network, production_params) in &production.networks {
        let continuation_params = continuation.networks.entry(network.clone()).or_default();
        for name in &preserved {
            if let Some(initial_block) = production_params.initial_blocks.get(name) {
                continuation_params
                    .initial_blocks
                    .insert(name.clone(), *initial_block);
            }
            if let Some(params) = production_params.params.get(name) {
                continuation_params
                    .params
                    .insert(name.clone(), params.clone());
            }
        }
    }

    continuation.sink_module = FINAL_OUTPUT_MODULE.to_string();

    let mut encoded = Vec::with_capacity(continuation.encoded_len());
    continuation.encode(&mut encoded)?;
    fs::write(output_path, encoded)?;

    preserved.sort();
    added.sort();
    println!("created {}", output_path.display());
    println!("preserved exact production modules: {}", preserved.len());
    println!("added template-only modules: {}", added.len());
    println!("sink module: {FINAL_OUTPUT_MODULE}");
    println!("backfill module: {BACKFILL_OUTPUT_MODULE}");
    Ok(())
}
