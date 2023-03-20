use ckb_types::{core::TransactionView, H256};

use crate::{
    parser::Parser,
    traits::SecpCkbRawKeySigner,
    tx_builder::{
        builder::{impl_default_builder, BaseTransactionBuilder, CkbTransactionBuilder},
        TxBuilderError,
    },
    unlock::{OmniLockScriptSigner, OmniLockUnlocker, OmniUnlockMode, ScriptUnlocker},
    Address, NetworkInfo, ScriptGroup,
};

use std::{
    ops::{Deref, DerefMut},
    str::FromStr,
};

use super::*;

pub struct DefaultOmnilockBuilder {
    pub base_builder: BaseTransactionBuilder,
    pub cfg: OmniLockConfig,
    pub rce_cells: Option<Vec<OutPoint>>,
    unlock_mode: OmniUnlockMode,
}

impl DefaultOmnilockBuilder {
    /// Make a builder with empty reciver list
    pub fn new_mainnet(sender_addr: &str, cfg: OmniLockConfig) -> Result<Self, TxBuilderError> {
        Self::new(NetworkInfo::mainnet(), sender_addr, cfg)
    }

    pub fn new(
        network_info: NetworkInfo,
        sender_addr: &str,
        cfg: OmniLockConfig,
    ) -> Result<Self, TxBuilderError> {
        let sender_address =
            Address::from_str(sender_addr).map_err(TxBuilderError::AddressFormat)?;
        Self::new_with_address(network_info, sender_address, cfg)
    }
    pub fn new_with_address(
        network_info: NetworkInfo,
        sender_address: Address,
        cfg: OmniLockConfig,
    ) -> Result<Self, TxBuilderError> {
        let mut base_builder =
            BaseTransactionBuilder::new_with_address(network_info, sender_address)?;
        let unlock_mode = OmniUnlockMode::default();
        let placeholder_witness = cfg.placeholder_witness(unlock_mode)?;
        base_builder.set_sender_placeholder_witness(placeholder_witness);
        Ok(Self {
            base_builder,
            cfg,
            rce_cells: None,
            unlock_mode,
        })
    }

    pub fn set_unlock_mode(&mut self, unlock_mode: OmniUnlockMode) -> Result<(), TxBuilderError> {
        let placeholder_witness = self.cfg.placeholder_witness(unlock_mode)?;
        self.base_builder
            .set_sender_placeholder_witness(placeholder_witness);
        Ok(())
    }

    /// add a sighash unlocker with private keys
    pub fn add_sighash_unlocker_from_str<T: AsRef<str>>(
        &mut self,
        keys: &[T],
    ) -> Result<(), TxBuilderError> {
        let mut sign_keys = vec![];
        for key in keys.iter() {
            let sender_key = H256::parse(key.as_ref()).map_err(TxBuilderError::KeyFormat)?;
            sign_keys.push(sender_key);
        }
        self.add_sighash_unlocker(sign_keys)
    }

    /// add a sighash unlocker with private keys
    pub fn add_sighash_unlocker(&mut self, sign_keys: Vec<H256>) -> Result<(), TxBuilderError> {
        let mut secrect_keys = vec![];
        for key in sign_keys.iter() {
            let sender_key = secp256k1::SecretKey::from_slice(key.as_bytes())
                .map_err(|e| TxBuilderError::KeyFormat(e.to_string()))?;
            secrect_keys.push(sender_key);
        }
        self.add_sighash_unlocker_from_secrect_keys(secrect_keys)
    }

    /// add sighash unlocker
    pub fn add_sighash_unlocker_from_secrect_keys(
        &mut self,
        secrect_keys: Vec<secp256k1::SecretKey>,
    ) -> Result<(), TxBuilderError> {
        let signer = SecpCkbRawKeySigner::new_with_secret_keys(secrect_keys);
        let omnilock_signer =
            OmniLockScriptSigner::new(Box::new(signer), self.cfg.clone(), self.unlock_mode);
        let omnilock_unlocker = OmniLockUnlocker::new(omnilock_signer, self.cfg.clone());
        let omnilock_script_id =
            super::get_default_script_id(self.base_builder.network_info.network_type);
        self.unlockers.insert(
            omnilock_script_id,
            Box::new(omnilock_unlocker) as Box<dyn ScriptUnlocker>,
        );
        Ok(())
    }
}

impl From<&DefaultOmnilockBuilder> for OmniLockTransferBuilder {
    fn from(val: &DefaultOmnilockBuilder) -> Self {
        OmniLockTransferBuilder {
            outputs: val.outputs.clone(),
            cfg: val.cfg.clone(),
            rce_cells: val.rce_cells.clone(),
        }
    }
}
impl_default_builder!(DefaultOmnilockBuilder, OmniLockTransferBuilder);
