use std::collections::HashSet;

use ckb_types::{
    bytes::Bytes,
    core::{DepType, ScriptHashType, TransactionBuilder, TransactionView},
    h256,
    packed::{Byte32, CellDep, CellInput, CellOutput, OutPoint},
    prelude::*,
    H256,
};

use super::{TxBuilder, TxBuilderError};
use crate::{
    constants::{MULTISIG_TYPE_HASH, SIGHASH_TYPE_HASH},
    traits::{CellCollector, CellDepResolver, HeaderDepResolver, TransactionDependencyProvider},
    unlock::OmniLockConfig,
    Address, AddressPayload,
};
use crate::{types::ScriptId, NetworkType};

pub use builder::DefaultOmnilockBuilder;

/// A builder to build an omnilock transfer transaction.
pub struct OmniLockTransferBuilder {
    pub outputs: Vec<(CellOutput, Bytes)>,
    pub cfg: OmniLockConfig,
    pub rce_cells: Option<Vec<OutPoint>>,
}

impl OmniLockTransferBuilder {
    pub fn new(
        outputs: Vec<(CellOutput, Bytes)>,
        cfg: OmniLockConfig,
        rce_cells: Option<Vec<OutPoint>>,
    ) -> OmniLockTransferBuilder {
        OmniLockTransferBuilder {
            outputs,
            cfg,
            rce_cells,
        }
    }
}

impl TxBuilder for OmniLockTransferBuilder {
    fn build_base(
        &self,
        _cell_collector: &mut dyn CellCollector,
        cell_dep_resolver: &dyn CellDepResolver,
        _header_dep_resolver: &dyn HeaderDepResolver,
        tx_dep_provider: &dyn TransactionDependencyProvider,
    ) -> Result<TransactionView, TxBuilderError> {
        #[allow(clippy::mutable_key_type)]
        let mut cell_deps = HashSet::new();
        #[allow(clippy::mutable_key_type)]
        let mut inputs = HashSet::new();
        let mut outputs = Vec::new();
        let mut outputs_data = Vec::new();
        for (output, output_data) in &self.outputs {
            outputs.push(output.clone());
            outputs_data.push(output_data.pack());
            if let Some(type_script) = output.type_().to_opt() {
                let script_id = ScriptId::from(&type_script);
                if !script_id.is_type_id() {
                    let cell_dep = cell_dep_resolver
                        .resolve(&type_script)
                        .ok_or(TxBuilderError::ResolveCellDepFailed(type_script))?;
                    cell_deps.insert(cell_dep);
                }
            }
        }
        if let Some(admin_cfg) = self.cfg.get_admin_config() {
            if let Some(rce_cells) = self.rce_cells.as_ref() {
                if admin_cfg.rce_in_input() {
                    for cell in rce_cells {
                        let input = CellInput::new_builder()
                            .previous_output(cell.clone())
                            .build();
                        inputs.insert(input);
                        let cell_output = tx_dep_provider.get_cell(cell)?;
                        // extract lock dep
                        let lock = cell_output.lock();
                        if let Some(cell_dep) = cell_dep_resolver.resolve(&lock) {
                            cell_deps.insert(cell_dep);
                        }
                        // extract type dependency
                        if let Some(type_) = cell_output.type_().to_opt() {
                            if let Some(cell_dep) = cell_dep_resolver.resolve(&type_) {
                                cell_deps.insert(cell_dep);
                            }
                        }
                    }
                } else {
                    for cell in rce_cells {
                        let cell_dep = CellDep::new_builder()
                            .out_point(cell.clone())
                            .dep_type(DepType::Code.into())
                            .build();
                        cell_deps.insert(cell_dep);
                    }
                }
            }
        }
        let id_flag = if let Some(admin_cfg) = self.cfg.get_admin_config() {
            admin_cfg.get_auth().flag()
        } else {
            self.cfg.id().flag()
        };
        match id_flag {
            crate::unlock::IdentityFlag::PubkeyHash |
            // ethereum only need secp256k1_data, and sighash group_dep contains it.
            crate::unlock::IdentityFlag::Ethereum => {
                let type_script = ScriptId::new_type(SIGHASH_TYPE_HASH).dummy_script();
                if let Some(cell_dep) = cell_dep_resolver.resolve(&type_script) {
                    cell_deps.insert(cell_dep);
                }
            }
            crate::unlock::IdentityFlag::Multisig => {
                let type_script = ScriptId::new_type(MULTISIG_TYPE_HASH).dummy_script();
                if let Some(cell_dep) = cell_dep_resolver.resolve(&type_script) {
                    cell_deps.insert(cell_dep);
                }
            } ,
            crate::unlock::IdentityFlag::OwnerLock => {},
            _ => todo!(),
        }
        Ok(TransactionBuilder::default()
            .set_cell_deps(cell_deps.into_iter().collect())
            .set_outputs(outputs)
            .set_inputs(inputs.into_iter().collect())
            .set_outputs_data(outputs_data)
            .build())
    }
}

pub const OMNILOCK_TYPE_HASH_MAINNET: H256 =
    h256!("0x9b819793a64463aed77c615d6cb226eea5487ccfc0783043a587254cda2b6f26");
pub const OMNILOCK_TX_HASH_MAINNET: H256 =
    h256!("0xdfdb40f5d229536915f2d5403c66047e162e25dedd70a79ef5164356e1facdc8");
pub const OMNILOCK_TX_INDEX_MAINNET: u32 = 0x0;
/// anyone can pay script testnet code hash
pub const OMNILOCK_TYPE_HASH_TESTNET: H256 =
    h256!("0xf329effd1c475a2978453c8600e1eaf0bc2087ee093c3ee64cc96ec6847752cb");
pub const OMNILOCK_TX_HASH_TESTNET: H256 =
    h256!("0x27b62d8be8ed80b9f56ee0fe41355becdb6f6a40aeba82d3900434f43b1c8b60");
pub const OMNILOCK_TX_INDEX_TESTNET: u32 = 0x0;

/// Add default acp cell dependencies, the dependent cells are metioned in the RFC.
pub fn add_default_omnilock_dep(dep_resolver: &mut dyn CellDepResolver, network_type: NetworkType) {
    let (code_hash, tx_hash, idx) = if network_type == NetworkType::Mainnet {
        (
            OMNILOCK_TYPE_HASH_MAINNET,
            OMNILOCK_TX_HASH_MAINNET,
            OMNILOCK_TX_INDEX_MAINNET,
        )
    } else if network_type == NetworkType::Testnet {
        (
            OMNILOCK_TYPE_HASH_TESTNET,
            OMNILOCK_TX_HASH_TESTNET,
            OMNILOCK_TX_INDEX_TESTNET,
        )
    } else {
        return;
    };

    let out_point = OutPoint::new(Byte32::from_slice(tx_hash.as_bytes()).unwrap(), idx);
    let cell_dep = CellDep::new_builder()
        .out_point(out_point)
        .dep_type(DepType::Code.into())
        .build();
    let script_id = ScriptId::new_type(code_hash);
    dep_resolver.insert(script_id, cell_dep);
}

pub fn get_default_script_id(network_type: NetworkType) -> ScriptId {
    let code_hash = if network_type == NetworkType::Mainnet {
        OMNILOCK_TYPE_HASH_MAINNET
    } else if network_type == NetworkType::Testnet {
        OMNILOCK_TYPE_HASH_TESTNET
    } else {
        panic!("can only handle mainnet and testnet");
    };
    ScriptId::new_type(code_hash)
}

pub fn build_omnilock_addr(network_type: NetworkType, config: &OmniLockConfig) -> Address {
    let args = config.build_args();
    let code_hash = if network_type == NetworkType::Mainnet {
        OMNILOCK_TYPE_HASH_MAINNET
    } else if network_type == NetworkType::Testnet {
        OMNILOCK_TYPE_HASH_TESTNET
    } else {
        panic!("can only handle mainnet and testnet");
    };
    let address_payload = AddressPayload::new_full(ScriptHashType::Type, code_hash.pack(), args);
    Address::new(network_type, address_payload, true)
}

mod builder;
