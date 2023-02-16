use ckb_types::{core::TransactionView, H256};

use crate::{
    parser::Parser,
    tx_builder::{
        builder::{impl_default_builder, BaseTransactionBuilder, CkbTransactionBuilder},
        TxBuilderError,
    },
    Address, NetworkInfo, ScriptGroup,
};

use std::{
    ops::{Deref, DerefMut},
    str::FromStr,
};

use super::*;

pub struct DefaultDaoDepositBuilder {
    pub base_builder: BaseTransactionBuilder,
    /// The deposit targets
    pub receivers: Vec<DaoDepositReceiver>,
}

impl DefaultDaoDepositBuilder {
    /// Make a builder with empty reciver list
    pub fn new(network_info: NetworkInfo, sender_addr: &str) -> Result<Self, TxBuilderError> {
        Ok(Self {
            base_builder: BaseTransactionBuilder::new(network_info, sender_addr)?,
            receivers: Default::default(),
        })
    }
    pub fn new_with_address(
        network_info: NetworkInfo,
        sender_address: Address,
    ) -> Result<Self, TxBuilderError> {
        Ok(Self {
            base_builder: BaseTransactionBuilder::new_with_address(network_info, sender_address)?,
            receivers: Default::default(),
        })
    }

    pub fn add_dao_output_str(
        &mut self,
        receiver_addr: &str,
        capacity: u64,
    ) -> Result<(), TxBuilderError> {
        let receiver_addr =
            Address::from_str(receiver_addr).map_err(TxBuilderError::AddressFormat)?;
        self.add_dao_output_addr(receiver_addr, capacity);
        Ok(())
    }

    pub fn add_dao_output_addr(&mut self, address: Address, capacity: u64) {
        let script = Script::from(address.payload());
        self.add_dao_output(script, capacity);
    }

    pub fn add_dao_output(&mut self, lock_script: Script, capacity: u64) {
        self.receivers
            .push(DaoDepositReceiver::new(lock_script, capacity));
    }
}

impl From<&DefaultDaoDepositBuilder> for DaoDepositBuilder {
    fn from(val: &DefaultDaoDepositBuilder) -> Self {
        DaoDepositBuilder {
            receivers: val.receivers.clone(),
        }
    }
}
impl_default_builder!(DefaultDaoDepositBuilder, DaoDepositBuilder);

pub struct DefaultDaoWithdrawPhase1Builder {
    pub base_builder: BaseTransactionBuilder,
    pub items: Vec<DaoPrepareItem>,
}

impl DefaultDaoWithdrawPhase1Builder {
    /// Make a builder with empty reciver list
    pub fn new(network_info: NetworkInfo, sender_addr: &str) -> Result<Self, TxBuilderError> {
        Ok(Self {
            base_builder: BaseTransactionBuilder::new(network_info, sender_addr)?,
            items: Default::default(),
        })
    }

    pub fn new_with_address(
        network_info: NetworkInfo,
        sender_address: Address,
    ) -> Result<Self, TxBuilderError> {
        Ok(Self {
            base_builder: BaseTransactionBuilder::new_with_address(network_info, sender_address)?,
            items: Default::default(),
        })
    }

    pub fn add_simple_input(
        &mut self,
        tx_hash: &str,
        index: u32,
        lock_script: Option<Script>,
    ) -> Result<(), TxBuilderError> {
        let hash =
            H256::parse(tx_hash).map_err(|e| TxBuilderError::InvalidParameter(anyhow!("{}", e)))?;
        let out_point = OutPoint::new_builder()
            .tx_hash(hash.pack())
            .index(index.pack())
            .build();
        let cell_input = CellInput::new_builder().previous_output(out_point).build();
        self.add_input_item(cell_input, lock_script);
        Ok(())
    }

    pub fn add_input_item(&mut self, input: CellInput, lock_script: Option<Script>) {
        self.items.push(DaoPrepareItem { input, lock_script })
    }

    pub fn add_item(&mut self, item: DaoPrepareItem) {
        self.items.push(item);
    }

    pub fn add_items(&mut self, items: &mut Vec<DaoPrepareItem>) {
        self.items.append(items);
    }
}

impl From<&DefaultDaoWithdrawPhase1Builder> for DaoPrepareBuilder {
    fn from(val: &DefaultDaoWithdrawPhase1Builder) -> Self {
        DaoPrepareBuilder {
            items: val.items.clone(),
        }
    }
}
impl_default_builder!(DefaultDaoWithdrawPhase1Builder, DaoPrepareBuilder);
