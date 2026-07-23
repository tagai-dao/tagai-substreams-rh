#![allow(dead_code)]

#[path = "../src/pb/mod.rs"]
mod pb;

use pb::sf::substreams::v1::{Binary, Module, Modules, Package};
use prost::Message;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env, fs,
    path::Path,
};

const UNIFIED_OUTPUT_MODULE: &str = "db_out";
const BASKET_MODULES: [&str; 3] = [
    "map_basket_registry_events",
    "store_basket_addresses",
    "map_basket_events",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModuleSource {
    Template,
    Legacy,
    Basket,
}

fn usage() -> &'static str {
    "usage: make_unified_package \
<legacy-continuation.spkg> <basket.spkg> <combined-template.spkg> <output.spkg>"
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
    let legacy_path = args.next().ok_or_else(usage)?;
    let basket_path = args.next().ok_or_else(usage)?;
    let template_path = args.next().ok_or_else(usage)?;
    let output_path = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage().into());
    }

    let output_path = Path::new(&output_path);
    for input in [&legacy_path, &basket_path, &template_path] {
        if Path::new(input) == output_path {
            return Err("output path must differ from every input path".into());
        }
    }

    let legacy = Package::decode(fs::read(&legacy_path)?.as_slice())?;
    let basket = Package::decode(fs::read(&basket_path)?.as_slice())?;
    let mut unified = Package::decode(fs::read(&template_path)?.as_slice())?;

    if legacy.network != basket.network || legacy.network != unified.network {
        return Err(format!(
            "network mismatch: legacy={:?}, basket={:?}, template={:?}",
            legacy.network, basket.network, unified.network
        )
        .into());
    }

    let legacy_modules = modules(&legacy)?;
    let basket_modules = modules(&basket)?;
    let legacy_by_name = module_map(&legacy)?;
    let basket_by_name = module_map(&basket)?;
    let basket_names = HashSet::<&str>::from_iter(BASKET_MODULES);

    let unified_modules = unified
        .modules
        .as_mut()
        .ok_or("combined template does not contain a modules section")?;
    if !unified_modules
        .modules
        .iter()
        .any(|module| module.name == UNIFIED_OUTPUT_MODULE)
    {
        return Err(format!(
            "combined template is missing output module {UNIFIED_OUTPUT_MODULE:?}"
        )
        .into());
    }

    // Keep the template binaries because the unified db_out lives there.
    // Append the exact legacy and Basket binaries, then point every upstream
    // module at its original binary content. Substreams module hashes resolve
    // the binary content rather than relying on a shared rebuild.
    let mut destination_binaries = unified_modules.binaries.clone();
    let mut legacy_binary_indexes = HashMap::new();
    let mut basket_binary_indexes = HashMap::new();
    let mut copied_from = BTreeMap::<String, ModuleSource>::new();

    for destination_module in &mut unified_modules.modules {
        let name = destination_module.name.clone();
        let source = if name == UNIFIED_OUTPUT_MODULE || name.starts_with("ethcommon:") {
            ModuleSource::Template
        } else if basket_names.contains(name.as_str()) {
            ModuleSource::Basket
        } else {
            ModuleSource::Legacy
        };

        *destination_module = match source {
            ModuleSource::Template => destination_module.clone(),
            ModuleSource::Legacy => {
                let source_module = legacy_by_name
                    .get(name.as_str())
                    .ok_or_else(|| format!("legacy package is missing module {name:?}"))?;
                copy_module(
                    source_module,
                    legacy_modules,
                    &mut destination_binaries,
                    &mut legacy_binary_indexes,
                )?
            }
            ModuleSource::Basket => {
                let source_module = basket_by_name
                    .get(name.as_str())
                    .ok_or_else(|| format!("Basket package is missing module {name:?}"))?;
                copy_module(
                    source_module,
                    basket_modules,
                    &mut destination_binaries,
                    &mut basket_binary_indexes,
                )?
            }
        };
        copied_from.insert(name, source);
    }
    unified_modules.binaries = destination_binaries;

    // Keep the human-readable block-filter map consistent with the copied
    // module definitions. The executable filter is already stored on Module.
    for (name, source) in &copied_from {
        let source_package = match source {
            ModuleSource::Template => continue,
            ModuleSource::Legacy => &legacy,
            ModuleSource::Basket => &basket,
        };
        match source_package.block_filters.get(name) {
            Some(filter) => {
                unified.block_filters.insert(name.clone(), filter.clone());
            }
            None => {
                unified.block_filters.remove(name);
            }
        }
    }

    unified.sink_module = UNIFIED_OUTPUT_MODULE.to_string();

    let mut encoded = Vec::with_capacity(unified.encoded_len());
    unified.encode(&mut encoded)?;
    fs::write(output_path, encoded)?;

    let legacy_count = copied_from
        .values()
        .filter(|source| **source == ModuleSource::Legacy)
        .count();
    let basket_count = copied_from
        .values()
        .filter(|source| **source == ModuleSource::Basket)
        .count();
    let template_count = copied_from
        .values()
        .filter(|source| **source == ModuleSource::Template)
        .count();

    println!("created {}", output_path.display());
    println!(
        "module sources: legacy={legacy_count}, basket={basket_count}, template={template_count}"
    );
    println!("preserved upstream WASM binaries; unified sink module: {UNIFIED_OUTPUT_MODULE}");
    Ok(())
}
