use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
};

use ethers::prelude::*;
use ethers::{abi::Address, types::H256};
use eyre::Result;
use serde::Serialize;

use super::transforms;

extern "C" {
    fn GetWitnessEx(str: *const c_char) -> *const c_char;
}

#[derive(Serialize, Debug, Clone)]
struct TrieModification {
    #[serde(rename = "Type")]
    typ: u8,
    #[serde(rename = "Key")]
    key: H256,
    #[serde(rename = "Value")]
    value: H256,
    #[serde(rename = "Address")]
    address: Address,
    #[serde(rename = "Nonce")]
    nonce: u64,
    #[serde(rename = "Balance")]
    balance: serde_json::Number,
    #[serde(rename = "CodeHash")]
    code_hash: Vec<u8>,
}
#[derive(Debug, Serialize)]
struct GetWitnessExRequest {
    #[serde(rename = "BlockNum")]
    block_no: u64,
    #[serde(rename = "Mods")]
    mods: Vec<TrieModification>,
    #[serde(rename = "NodeUrl")]
    node_url: String,
}

pub fn generate(
    block_no: U64,
    trns: Vec<transforms::TrieModification>,
    provider: &str,
) -> Result<String> {
    let trns: Vec<_> = trns
        .into_iter()
        .map(|m| TrieModification {
            typ: m.typ as u8,
            key: m.key,
            value: {
                let mut bytes = [0u8; 32];
                m.value.to_big_endian(&mut bytes);
                H256::from_slice(&bytes)
            },
            address: m.address,
            nonce: m.nonce.as_u64(),
            balance: serde_json::Number::from_string_unchecked(format!("{}", m.balance)),
            code_hash: m.code_hash.as_bytes().to_vec(),
        })
        .collect();

    let req = GetWitnessExRequest {
        block_no: block_no.as_u64(),
        mods: trns,
        node_url: provider.to_string(),
    };

    let json = serde_json::to_string(&req)?;
    println!("request: {}", &json);
    println!("------------------------------");
    let c_config = CString::new(json).expect("invalid config");

    let result = unsafe { GetWitnessEx(c_config.as_ptr() as *const i8) };
    let c_str = unsafe { CStr::from_ptr(result) };
    let json = c_str.to_str().expect("Error translating from library");
    Ok(json.to_string())
}
