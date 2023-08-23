use halo2_proofs::{dev::MockProver, halo2curves::bn256::Fr};
use zkevm_circuits::mpt_circuit::witness_row::{Node, self};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::fs::File;
use std::io::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Result;

mod circuit;

extern "C" {
    fn GetWitness(str: *const c_char) -> *const c_char;
}

/// Loads an MPT proof from disk
pub fn load_proof(path: &str) -> Vec<Node> {
    let file = std::fs::File::open(path);
    let reader = std::io::BufReader::new(file.unwrap());
    let mut nodes: Vec<Node> = serde_json::from_reader(reader).unwrap();

    // Add the address and the key to the list of values in the Account and Storage nodes
    for node in nodes.iter_mut() {
        if node.account.is_some() {
            let account = node.account.clone().unwrap();
            node.values.push([vec![148], account.address].concat());
            node.values.push([vec![160], account.key].concat());
        }
        if node.storage.is_some() {
            let storage: witness_row::StorageNode = node.storage.clone().unwrap();
            node.values.push([vec![160], storage.address].concat());
            node.values.push([vec![160], storage.key].concat());
        }
    }
    nodes
}

#[derive(Serialize, Deserialize)]
struct GetWitnessRequest {
    #[serde(rename = "NodeUrl")]
    node_url: String,
    #[serde(rename = "BlockNum")]
    block_num: u64,
    #[serde(rename = "Addr")]
    addr: String,
    #[serde(rename = "Keys")]
    keys: Vec<String>,
    #[serde(rename = "Values")]
    values: Vec<String>,
}

const URL : &str ="https://mainnet.infura.io/v3/9aa3d95b3bc440fa88ea12eaa4456161";

fn main() -> Result<()> {
    let data = r#"
        {
            "NodeUrl": "https://mainnet.infura.io/v3/9aa3d95b3bc440fa88ea12eaa4456161",
            "BlockNum": 14359865,
            "Addr": "0x4E5B2e1dc63F6b91cb6Cd759936495434C7e972F",
            "Keys": ["0x12"],
            "Values": ["0x1123e2"]
        }"#;

    let c_config = CString::new(data).expect("invalid config");

    let result = unsafe { GetWitness(c_config.as_ptr()) };
    let c_str = unsafe { CStr::from_ptr(result) };
    let json = c_str.to_str().expect("Error translating from library");
    let filename = "./mpt.json";

    println!("{json}");

    // write json into filename and read it back as list of nodes
    File::create(filename).and_then(|mut f| f.write_all(json.as_bytes())).unwrap();

    circuit::verify_proof(filename);

    Ok(())
}
