#![allow(dead_code)]

#[path = "../src/pb/mod.rs"]
mod pb;

use pb::sf::substreams::v1::{Binary, Module, Package};
use prost::Message;
use std::{env, fs, path::Path};

const TARGET_MODULE: &str = "index_broker_db_out";
const FINAL_OUTPUT_MODULE: &str = "v11_continuation_db_out";
const ROOT_PACKAGE_NAME: &str = "tiptag_substreams";

fn usage() -> &'static str {
    "usage: make_index_broker_output_hotfix \
<exact-cutover.spkg> <fixed-template.spkg> <output.spkg>"
}

fn find_module<'a>(package: &'a Package, name: &str) -> Result<&'a Module, String> {
    package
        .modules
        .as_ref()
        .ok_or_else(|| "SPKG does not contain a modules section".to_string())?
        .modules
        .iter()
        .find(|module| module.name == name)
        .ok_or_else(|| format!("SPKG is missing module {name:?}"))
}

fn module_binary(package: &Package, module: &Module) -> Result<Binary, String> {
    package
        .modules
        .as_ref()
        .ok_or_else(|| "SPKG does not contain a modules section".to_string())?
        .binaries
        .get(module.binary_index as usize)
        .cloned()
        .ok_or_else(|| {
            format!(
                "module {:?} refers to missing binary index {}",
                module.name, module.binary_index
            )
        })
}

fn compatible_definition(base: &Module, candidate: &Module) -> bool {
    let mut normalized_base = base.clone();
    let mut normalized_candidate = candidate.clone();

    // The cutover package deliberately starts this output later than the
    // source manifest. Its boundary must remain unchanged. Only the compiled
    // binary is allowed to differ in this hotfix.
    normalized_base.binary_index = 0;
    normalized_candidate.binary_index = 0;
    normalized_candidate.initial_block = normalized_base.initial_block;

    normalized_base == normalized_candidate
}

fn apply_hotfix(mut base: Package, candidate: &Package) -> Result<Package, String> {
    if base.sink_module != FINAL_OUTPUT_MODULE {
        return Err(format!(
            "base package sink must be {FINAL_OUTPUT_MODULE:?}, got {:?}",
            base.sink_module
        ));
    }
    if base.network != candidate.network {
        return Err(format!(
            "network mismatch: base={:?}, candidate={:?}",
            base.network, candidate.network
        ));
    }

    // Both modules must exist before any mutation. The final module keeps its
    // old binary and definition; its hash changes only because this target is
    // one of its inputs.
    find_module(&base, FINAL_OUTPUT_MODULE)?;
    let base_target = find_module(&base, TARGET_MODULE)?.clone();
    let candidate_target = find_module(candidate, TARGET_MODULE)?.clone();
    if !compatible_definition(&base_target, &candidate_target) {
        return Err(format!(
            "module {TARGET_MODULE:?} changed beyond its binary or cutover initial block"
        ));
    }
    let fixed_binary = module_binary(candidate, &candidate_target)?;

    let modules = base
        .modules
        .as_mut()
        .ok_or_else(|| "SPKG does not contain a modules section".to_string())?;
    let fixed_binary_index = modules.binaries.len() as u32;
    modules.binaries.push(fixed_binary);
    let target = modules
        .modules
        .iter_mut()
        .find(|module| module.name == TARGET_MODULE)
        .expect("target existence checked above");
    target.binary_index = fixed_binary_index;

    // Carry the fixed template's root release metadata into the derived SPKG.
    // Imported package metadata and every executable module definition remain
    // those of the exact cutover artifact.
    if let Some(candidate_meta) = candidate
        .package_meta
        .iter()
        .find(|meta| meta.name == ROOT_PACKAGE_NAME)
    {
        if let Some(base_meta) = base
            .package_meta
            .iter_mut()
            .find(|meta| meta.name == ROOT_PACKAGE_NAME)
        {
            *base_meta = candidate_meta.clone();
        }
    }

    Ok(base)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let base_path = args.next().ok_or_else(usage)?;
    let candidate_path = args.next().ok_or_else(usage)?;
    let output_path = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage().into());
    }
    if Path::new(&base_path) == Path::new(&output_path)
        || Path::new(&candidate_path) == Path::new(&output_path)
    {
        return Err("output path must differ from both input paths".into());
    }

    let base = Package::decode(fs::read(&base_path)?.as_slice())?;
    let candidate = Package::decode(fs::read(&candidate_path)?.as_slice())?;
    let base_target_initial_block = find_module(&base, TARGET_MODULE)?.initial_block;
    let package = apply_hotfix(base, &candidate)?;

    let mut encoded = Vec::with_capacity(package.encoded_len());
    package.encode(&mut encoded)?;
    fs::write(&output_path, encoded)?;

    println!("created {}", Path::new(&output_path).display());
    println!("replaced binary for: {TARGET_MODULE}");
    println!("preserved target initial block: {base_target_initial_block}");
    println!("preserved sink module definition: {FINAL_OUTPUT_MODULE}");
    println!("expected changed module hashes: {TARGET_MODULE}, {FINAL_OUTPUT_MODULE}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pb::sf::substreams::v1::{module, Modules, PackageMetadata};

    fn map_module(name: &str, binary_index: u32, initial_block: u64) -> Module {
        Module {
            name: name.to_string(),
            binary_index,
            binary_entrypoint: name.to_string(),
            inputs: Vec::new(),
            output: Some(module::Output {
                r#type: "proto:test.Output".to_string(),
            }),
            initial_block,
            block_filter: None,
            kind: Some(module::Kind::KindMap(module::KindMap {
                output_type: "proto:test.Output".to_string(),
            })),
        }
    }

    fn package(version: &str, binary: u8, target_initial_block: u64) -> Package {
        Package {
            modules: Some(Modules {
                modules: vec![
                    map_module(TARGET_MODULE, 0, target_initial_block),
                    map_module(FINAL_OUTPUT_MODULE, 0, target_initial_block),
                    map_module("unchanged_store", 0, 123),
                ],
                binaries: vec![Binary {
                    r#type: "wasm/rust-v1".to_string(),
                    content: vec![binary],
                }],
            }),
            package_meta: vec![PackageMetadata {
                version: version.to_string(),
                url: "https://example.invalid".to_string(),
                name: ROOT_PACKAGE_NAME.to_string(),
                doc: String::new(),
            }],
            network: "robinhood-mainnet".to_string(),
            sink_module: FINAL_OUTPUT_MODULE.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn replaces_only_the_target_binary_and_preserves_cutover_metadata() {
        let base = package("v0.5.2", 1, 53_869_281);
        let unchanged_before = find_module(&base, "unchanged_store").unwrap().clone();
        let final_before = find_module(&base, FINAL_OUTPUT_MODULE).unwrap().clone();
        let candidate = package("v0.5.3", 2, 52_436_657);

        let hotfix = apply_hotfix(base, &candidate).unwrap();
        let modules = hotfix.modules.as_ref().unwrap();
        let target = find_module(&hotfix, TARGET_MODULE).unwrap();

        assert_eq!(modules.binaries.len(), 2);
        assert_eq!(target.binary_index, 1);
        assert_eq!(target.initial_block, 53_869_281);
        assert_eq!(module_binary(&hotfix, target).unwrap().content, vec![2]);
        assert_eq!(
            find_module(&hotfix, "unchanged_store").unwrap(),
            &unchanged_before
        );
        assert_eq!(
            find_module(&hotfix, FINAL_OUTPUT_MODULE).unwrap(),
            &final_before
        );
        assert_eq!(hotfix.package_meta[0].version, "v0.5.3");
    }

    #[test]
    fn rejects_non_binary_target_changes() {
        let base = package("v0.5.2", 1, 53_869_281);
        let mut candidate = package("v0.5.3", 2, 52_436_657);
        find_module(&candidate, TARGET_MODULE).unwrap();
        candidate.modules.as_mut().unwrap().modules[0].binary_entrypoint =
            "different_entrypoint".to_string();

        let error = apply_hotfix(base, &candidate).unwrap_err();
        assert!(error.contains("changed beyond its binary"));
    }
}
