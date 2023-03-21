use std::{error::Error as StdErr, str::FromStr};

use ckb_sdk::{
    constants::ONE_CKB,
    parser::Parser,
    tx_builder::{
        builder::CkbTransactionBuilder,
        omni_lock::{build_omnilock_unlockers, DefaultOmnilockBuilder},
        unlock_tx,
    },
    unlock::{MultisigConfig, OmniLockConfig},
    Address, NetworkInfo,
};
use ckb_types::H256;

fn main() -> Result<(), Box<dyn StdErr>> {
    let network_info = NetworkInfo::testnet();
    let sender =  "ckt1qrejnmlar3r452tcg57gvq8patctcgy8acync0hxfnyka35ywafvkqgxhjvp3k9pf88upngryvuxc346q7fq5qmlqqlrhr0p";
    let receiver = sender;

    let sender_address = Address::from_str(sender).unwrap();
    let mut cfg = OmniLockConfig::from_addr(&sender_address).unwrap();
    let multisig_config = MultisigConfig::new_with_hash_str(
        &[
            "0x7336b0ba900684cb3cb00f0d46d4f64c0994a562",
            "0x5724c1e3925a5206944d753a6f3edaedf977d77f",
        ],
        0,
        2,
    )
    .unwrap();
    cfg.set_multisig_config(Some(multisig_config));

    let mut builder =
        DefaultOmnilockBuilder::new(network_info.clone(), sender, cfg.clone()).unwrap();
    builder.add_output_raw(receiver, 128 * ONE_CKB).unwrap();
    builder
        .add_sighash_unlocker_from_str(&[
            "0x7438f7b35c355e3d2fb9305167a31a72d22ddeafb80a21cc99ff6329d92e8087",
        ])
        .unwrap();

    let (tx, unsigned_group) = builder.build_unlocked().unwrap();

    // second sign
    let sender_key =
        H256::parse("0x4fd809631a6aa6e3bb378dd65eae5d71df895a82c91a615a1e8264741515c79c").unwrap();
    let sender_key = secp256k1::SecretKey::from_slice(sender_key.as_bytes()).unwrap();
    let unlockers = build_omnilock_unlockers(
        vec![sender_key],
        cfg,
        network_info.network_type,
        ckb_sdk::unlock::OmniUnlockMode::Normal,
    );

    let (new_tx, _new_still_locked_groups) =
        unlock_tx(tx, builder.tx_dep_provider.as_ref(), &unlockers)?;
    let tx = new_tx;

    println!(
        "> tx: {}",
        serde_json::to_string_pretty(&ckb_jsonrpc_types::TransactionView::from(tx.clone()))
            .unwrap()
    );

    println!("unsigned_group len:{}", unsigned_group.len());
    let tx_hash = builder.send_transaction(tx)?;
    // example tx_hash : f4e6d937ebf3fc7cc3eaf4fd5d4c73efd5e66ea7388e7ccb6244afb9e81669d4
    // example tx_hash : 0dcf98835bc36d26cf4c5be4b244361e99745469c18545dd814953b779a31eff
    println!("tx {} sent", tx_hash);
    Ok(())
}
