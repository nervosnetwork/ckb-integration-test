use crate::{Node, User};
use ckb_crypto::secp::Pubkey;
use ckb_hash::blake2b_256;
use ckb_types::core::cell::CellMeta;
use ckb_types::{
    bytes::Bytes,
    core::{DepType, ScriptHashType, TransactionView},
    h256,
    packed::{Byte32, CellDep, OutPoint, Script, WitnessArgs},
    prelude::*,
    H160, H256,
};

pub const GENESIS_DEP_GROUP_TRANSACTION_INDEX: usize = 1;
pub const GENESIS_SIGHASH_ALL_DEP_GROUP_CELL_INDEX: usize = 0;
pub const SIGHASH_ALL_TYPE_HASH: H256 =
    h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");

impl User {
    pub fn single_secp256k1_lock_hash(&self) -> Byte32 {
        self.single_secp256k1_lock_script().calc_script_hash()
    }

    pub fn single_secp256k1_lock_script(&self) -> Script {
        Script::new_builder()
            .hash_type(ScriptHashType::Type.into())
            .code_hash(SIGHASH_ALL_TYPE_HASH.pack())
            .args(self.single_secp256k1_address().0.pack())
            .build()
    }

    pub fn single_secp256k1_address(&self) -> H160 {
        let pubkey = self.single_secp256k1_pubkey();
        H160::from_slice(&blake2b_256(pubkey.serialize())[0..20]).unwrap()
    }

    pub fn single_secp256k1_out_point(&self) -> OutPoint {
        OutPoint::new_builder()
            .tx_hash(
                self.genesis_block
                    .transaction(GENESIS_DEP_GROUP_TRANSACTION_INDEX)
                    .expect("index genesis dep-group transaction")
                    .hash(),
            )
            .index(GENESIS_SIGHASH_ALL_DEP_GROUP_CELL_INDEX.pack())
            .build()
    }

    pub fn single_secp256k1_cell_dep(&self) -> CellDep {
        CellDep::new_builder()
            .out_point(self.single_secp256k1_out_point())
            .dep_type(DepType::DepGroup.into())
            .build()
    }

    pub fn single_secp256k1_pubkey(&self) -> Pubkey {
        if let Some(ref privkey) = self.single_secp256k1_privkey {
            privkey.pubkey().unwrap()
        } else {
            unreachable!("single_secp256k1 unset")
        }
    }

    pub fn single_secp256k1_signed_witness(&self, tx: &TransactionView) -> WitnessArgs {
        if let Some(ref privkey) = self.single_secp256k1_privkey {
            let tx_hash = tx.hash();
            let mut blake2b = ckb_hash::new_blake2b();
            let mut message = [0u8; 32];
            blake2b.update(&tx_hash.raw_data());
            let witness_for_digest = WitnessArgs::new_builder()
                .lock(Some(Bytes::from(vec![0u8; 65])).pack())
                .build();
            let witness_len = witness_for_digest.as_bytes().len() as u64;
            blake2b.update(&witness_len.to_le_bytes());
            blake2b.update(&witness_for_digest.as_bytes());
            blake2b.finalize(&mut message);
            let message = H256::from(message);
            let sig = privkey.sign_recoverable(&message).expect("sign");
            WitnessArgs::new_builder()
                .lock(Some(Bytes::from(sig.serialize())).pack())
                .build()
            // .as_bytes()
            // .pack()
        } else {
            unreachable!("single_secp256k1 unset")
        }
    }

    pub fn get_spendable_single_secp256k1_cells(&self, node: &Node) -> Vec<CellMeta> {
        let live_out_points = node
            .indexer()
            .get_live_cells_by_lock_script(&self.single_secp256k1_lock_script())
            .expect("indexer get_live_cells_by_lock_script");
        live_out_points
            .into_iter()
            .filter_map(|out_point| {
                let cell_meta = node.get_cell_meta(out_point)?;
                if cell_meta.data_bytes == 0 {
                    Some(cell_meta)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }
}
