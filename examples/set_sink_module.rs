#![allow(dead_code)]

#[path = "../src/pb/mod.rs"]
mod pb;

use pb::sf::substreams::v1::Package;
use prost::Message;
use std::{env, fs, path::Path};

fn usage() -> &'static str {
    "usage: set_sink_module <input.spkg> <module-name> <output.spkg>"
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let input_path = args.next().ok_or_else(usage)?;
    let module_name = args
        .next()
        .ok_or_else(usage)?
        .into_string()
        .map_err(|_| "module name must be valid UTF-8")?;
    let output_path = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage().into());
    }
    if Path::new(&input_path) == Path::new(&output_path) {
        return Err("output path must differ from input path".into());
    }

    let mut package = Package::decode(fs::read(&input_path)?.as_slice())?;
    let module = package
        .modules
        .as_ref()
        .and_then(|modules| {
            modules
                .modules
                .iter()
                .find(|module| module.name == module_name)
        })
        .ok_or_else(|| format!("package does not contain module {module_name:?}"))?;
    let output_type = module
        .output
        .as_ref()
        .map(|output| output.r#type.as_str())
        .unwrap_or_default();
    if output_type != "proto:sf.substreams.sink.database.v1.DatabaseChanges" {
        return Err(format!(
            "module {module_name:?} has unsupported sink output {output_type:?}"
        )
        .into());
    }

    package.sink_module = module_name.clone();
    let mut encoded = Vec::with_capacity(package.encoded_len());
    package.encode(&mut encoded)?;
    fs::write(&output_path, encoded)?;

    println!(
        "created {} with sink module {}",
        Path::new(&output_path).display(),
        module_name
    );
    Ok(())
}
