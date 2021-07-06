use ckb_crypto::secp::{Privkey, Pubkey};
use ckb_hash::blake2b_256;
use ckb_types::{
    core::ScriptHashType,
    h256,
    packed::{Byte32, Script},
    prelude::*,
    H160, H256,
};
use std::str::FromStr;

pub const SIGHASH_ALL_TYPE_HASH: H256 =
    h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");

#[derive(Clone)]
pub struct User {
    privkey: Privkey,
    pubkey: Pubkey,
    address: H160,
    lock_script: Script,
    lock_hash: Byte32,
}

impl User {
    pub fn new(privkey: &str) -> Self {
        let privkey = Privkey::from_str(privkey).unwrap_or_else(|err| {
            crate::prompt_and_exit!("failed to parse privkey, error: {}", err)
        });
        let pubkey = privkey.pubkey().unwrap();
        let address = H160::from_slice(&blake2b_256(pubkey.serialize())[0..20]).unwrap();
        let lock_script = Script::new_builder()
            .hash_type(ScriptHashType::Type.into())
            .code_hash(SIGHASH_ALL_TYPE_HASH.pack())
            .args(address.0.pack())
            .build();
        let lock_hash = lock_script.calc_script_hash();
        User {
            privkey,
            pubkey,
            address,
            lock_script,
            lock_hash,
        }
    }

    pub fn privkey(&self) -> &Privkey {
        &self.privkey
    }

    pub fn pubkey(&self) -> &Pubkey {
        &self.pubkey
    }

    pub fn address(&self) -> &H160 {
        &self.address
    }

    pub fn lock_script(&self) -> &Script {
        &self.lock_script
    }

    pub fn lock_hash(&self) -> &Byte32 {
        &self.lock_hash
    }
}
