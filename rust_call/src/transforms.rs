use std::sync::Arc;

use ethers::{
    abi::Address,
    prelude::{k256::ecdsa::SigningKey, SignerMiddleware},
    providers::{Http, Provider},
    signers::Wallet,
    types::{
        transaction::eip2930::{AccessList, AccessListItem},
        BlockId, BlockNumber, EIP1186ProofResponse, H256, U256, U64,
    },
};

use ethers::prelude::*;
use eyre::Result;
use num_enum::IntoPrimitive;

#[derive(Default, Debug, IntoPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum ProofType {
    #[default]
    Disabled = 0,
    NonceChanged = 1,
    BalanceChanged = 2,
    CodeHashChanged = 3,
    AccountDestructed = 4,
    AccountDoesNotExist = 5,
    StorageChanged = 6,
    StorageDoesNotExist = 7,
    AccountCreate = 8,
}

#[derive(Default, Debug, Clone)]
pub struct TrieModification {
    pub typ: ProofType,
    pub key: H256,
    pub value: U256,
    pub address: Address,
    pub nonce: U64,
    pub balance: U256,
    pub code_hash: H256,
}

impl TrieModification {
    fn create(proof: &EIP1186ProofResponse) -> Self {
        Self {
            typ: ProofType::AccountCreate,
            key: H256::zero(),
            value: U256::zero(),
            address: proof.address,
            nonce: proof.nonce,
            balance: proof.balance,
            code_hash: proof.code_hash,
        }
    }
    fn balance(balance: U256) -> Self {
        Self {
            typ: ProofType::BalanceChanged,
            balance,
            ..Default::default()
        }
    }
    fn nonce(nonce: U64) -> Self {
        Self {
            typ: ProofType::NonceChanged,
            nonce,
            ..Default::default()
        }
    }
    fn storage_changed(key: H256, value: U256) -> Self {
        Self {
            typ: ProofType::StorageChanged,
            key,
            value,
            ..Default::default()
        }
    }
    fn storage_does_not_exist(key: H256) -> Self {
        Self {
            typ: ProofType::StorageDoesNotExist,
            key,
            ..Default::default()
        }
    }
}

pub(crate) async fn get_transforms(
    client: Arc<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>,
    block: U64,
) -> Result<Vec<TrieModification>> {
    let mut mods = Vec::new();

    // get the changes in the block
    let access_list: AccessList = client
        .provider()
        .request("eth_getAccessListByNumber", vec![format!("0x{:x}", block)])
        .await?;

    for entry in access_list.0 {
        let AccessListItem {
            address,
            storage_keys,
        } = entry;

        let old = client
            .get_proof(
                address,
                storage_keys.clone(),
                Some(BlockId::Number(BlockNumber::Number(block - 1))),
            )
            .await?;

        let new = client
            .get_proof(
                address,
                storage_keys.clone(),
                Some(BlockId::Number(BlockNumber::Number(block))),
            )
            .await?;

        // if nothing changed, skip
        if old.balance == new.balance
            && old.nonce == new.nonce
            && old.code_hash == new.code_hash
            && old.storage_hash == new.storage_hash
        {
            continue;
        }

        // check for this address changes
        if (old.balance.is_zero() && !new.balance.is_zero())
            || (old.nonce.is_zero() && !new.nonce.is_zero())
        {
            mods.push(TrieModification::create(&new));
        } else {
            if old.balance != new.balance {
                mods.push(TrieModification::balance(new.balance));
            }
            if old.nonce != new.nonce {
                mods.push(TrieModification::nonce(new.nonce));
            }
        }

        for key in storage_keys {
            let old = old.storage_proof.iter().find(|p| p.key == key).unwrap();
            let new = new.storage_proof.iter().find(|p| p.key == key).unwrap();
            if !old.value.is_zero() && new.value.is_zero() {
                mods.push(TrieModification::storage_does_not_exist(key));
            } else {
                mods.push(TrieModification::storage_changed(key, new.value));
            }
        }
    }
    Ok(mods)
}
