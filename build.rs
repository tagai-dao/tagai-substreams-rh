use anyhow::{Ok, Result};
use regex::Regex;
use std::fs;
use substreams_ethereum::Abigen;

fn main() -> Result<(), anyhow::Error> {
    let file_names = [
        "abi/pump_contract.abi.json",
        "abi/token_contract.abi.json",
        "abi/ipshare_contract.abi.json",
        "abi/swap_hook.abi.json",
        "abi/walnut.abi.json",
        "abi/basket_registry.abi.json",
        "abi/basket_hook.abi.json",
        "abi/basket_router.abi.json",
        "abi/basket_token.abi.json",
        "abi/basket_auction.abi.json",
        "abi/basket_rebalance.abi.json",
    ];
    let file_output_names = [
        "src/abi/pump_contract.rs",
        "src/abi/token_contract.rs",
        "src/abi/ipshare_contract.rs",
        "src/abi/swap_hook.rs",
        "src/abi/walnut.rs",
        "src/abi/basket_registry.rs",
        "src/abi/basket_hook.rs",
        "src/abi/basket_router.rs",
        "src/abi/basket_token.rs",
        "src/abi/basket_auction.rs",
        "src/abi/basket_rebalance.rs",
    ];

    let mut i = 0;
    for f in file_names {
        let contents = fs::read_to_string(f).expect("Should have been able to read the file");

        // sanitize fields and attributes starting with an underscore
        let regex = Regex::new(r#"("\w+"\s?:\s?")_(\w+")"#).unwrap();
        let sanitized_abi_file = regex.replace_all(contents.as_str(), "${1}u_${2}");

        // sanitize fields and attributes with multiple consecutive underscores
        let re = Regex::new(r"_+").unwrap();

        let re_sanitized_abi_file =
            re.replace_all(&sanitized_abi_file, |caps: &regex::Captures| {
                let count = caps[0].len();
                let replacement = format!("{}_", "_u".repeat(count - 1));
                replacement
            });

        Abigen::from_bytes("Contract", re_sanitized_abi_file.as_bytes())?
            .generate()?
            .write_to_file(file_output_names[i])?;

        i = i + 1;
    }

    Ok(())
}
