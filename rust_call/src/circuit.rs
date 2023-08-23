use eyre::Result;
use halo2_proofs::{dev::MockProver, halo2curves::bn256::Fr};
use zkevm_circuits::mpt_circuit::witness_row::{self, Node};

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

pub fn verify_proof(filename: &str) -> Result<()> {
    let nodes = load_proof(filename);

    // get the number of rows in the witness
    let num_rows: usize = nodes.iter().map(|node| node.values.len()).sum();

    // populate the keccak data
    let mut keccak_data = vec![];
    for node in nodes.iter() {
        for k in node.keccak_data.iter() {
            keccak_data.push(k.clone());
        }
    }

    // verify the circuit
    let disable_preimage_check = nodes[0].start.clone().unwrap().disable_preimage_check;
    let degree = 14;
    let circuit = zkevm_circuits::mpt_circuit::MPTCircuit::<Fr> {
        nodes,
        keccak_data,
        degree,
        disable_preimage_check,
        _marker: std::marker::PhantomData,
    };

    let prover = MockProver::<Fr>::run(degree as u32, &circuit, vec![]).unwrap();
    assert_eq!(prover.verify_at_rows(0..num_rows, 0..num_rows,), Ok(()));

    println!("success!");

    Ok(())
}
