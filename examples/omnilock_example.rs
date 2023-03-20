use std::{error::Error as StdErr, str::FromStr};

use ckb_sdk::{
    constants::ONE_CKB,
    tx_builder::{builder::CkbTransactionBuilder, omni_lock::DefaultOmnilockBuilder},
    unlock::OmniLockConfig,
    Address, NetworkInfo,
};

fn main() -> Result<(), Box<dyn StdErr>> {
    let network_info = NetworkInfo::testnet();
    let sender =  "ckt1qrejnmlar3r452tcg57gvq8patctcgy8acync0hxfnyka35ywafvkqgqgpy7m88v3gxnn3apazvlpkkt32xz3tg5qq3kzjf3";
    let receiver = sender;

    let sender_address = Address::from_str(sender).unwrap();
    let cfg = OmniLockConfig::from_addr(&sender_address).unwrap();
    let mut builder = DefaultOmnilockBuilder::new(network_info, sender, cfg).unwrap();
    builder.add_output_raw(receiver, 128 * ONE_CKB).unwrap();
    builder
        .add_sighash_unlocker_from_str(&[
            "0x6c9ed03816e3111e49384b8d180174ad08e29feb1393ea1b51cef1c505d4e36a",
        ])
        .unwrap();

    let (tx, unsigned_group) = builder.build_unlocked().unwrap();

    println!(
        "> tx: {}",
        serde_json::to_string_pretty(&ckb_jsonrpc_types::TransactionView::from(tx.clone()))
            .unwrap()
    );

    println!("unsigned_group len:{}", unsigned_group.len());
    let tx_hash = builder.send_transaction(tx)?;
    // example tx_hash : 1c4934bf596fd632cac9409c4902f6e352581a09979c242ae00fd1906319cde0
    println!("tx {} sent", tx_hash);
    Ok(())
}
