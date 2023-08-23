use ethers::prelude::k256::ecdsa::SigningKey;
use ethers::providers::Middleware;
use ethers::{
    middleware::SignerMiddleware,
    prelude::*,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    utils::format_units,
};
use eyre::Result;
use std::fs::File;
use std::io::Write;
use std::{convert::TryFrom, sync::Arc, time::Duration};

mod le_circuit;
mod circuit;
mod contract;
mod transforms;
mod witness;

const PVK: &str = "7ccb34dc5fd31fd0aa7860de89a4adc37ccb34dc5fd31fd0aa7860de89a4adc3";
const PROVIDER_URL: &str = "http://localhost:8545";

type MM = SignerMiddleware<Provider<Http>, Wallet<SigningKey>>;

async fn new_client() -> Result<Arc<MM>> {
    let provider: Provider<Http> =
        Provider::<Http>::try_from(PROVIDER_URL)?.interval(Duration::from_millis(10u64));
    let chain_id = provider.get_chainid().await?.as_u64();

    let wallet = PVK.parse::<LocalWallet>()?;
    let client = Arc::new(SignerMiddleware::new(
        provider,
        wallet.with_chain_id(chain_id),
    ));
    let balance = client.get_balance(client.address(), None).await?;

    println!(
        "address {:?} , balance {}ETH",
        client.address(),
        format_units(balance, "ether")?
    );

    Ok(client)
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = new_client().await?;
    let contract = contract::Contract::deploy(client.clone()).await?;
    let block_no = contract.set(0xcafee.into(), 0xcafe.into()).await?;

    let trns = transforms::get_transforms(client.clone(), block_no).await?;
    let wtns = witness::generate(block_no, trns, PROVIDER_URL)?;

    let filename = "./mpt.json";
    File::create(filename)
        .and_then(|mut f| f.write_all(wtns.as_bytes()))
        .unwrap();

    circuit::verify_proof(filename)?;

    Ok(())
}
