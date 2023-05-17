use anyhow::anyhow;
use ckb_hash::new_blake2b;
use ckb_types::{
    core::{Capacity, ScriptHashType},
    packed::{CellInput, CellOutput, OutPoint, Script},
    prelude::*,
};

use crate::{
    constants,
    core::TransactionBuilder,
    traits::{
        DefaultHeaderDepResolver, DefaultTransactionDependencyProvider, HeaderDepResolver, LiveCell,
    },
    transaction::{builder::PrepareTransactionViewer, input::TransactionInput},
    tx_builder::TxBuilderError,
    NetworkInfo, ScriptGroup, ScriptId,
};

use super::{HandlerContext, ScriptHandler};

pub struct TypeIdHandler;

pub enum TypeIdAction {
    // user already add output, and not need to update an existing cell.
    Handled,
    // create a new output
    Create {
        lock: Script,
    },
    // update an exinsting output
    Update {
        input_point: OutPoint,
        rpc_url: String,
        // if it's none, use original lock script
        lock: Option<Script>,
    },
}

pub struct TypeIdContext {
    action: TypeIdAction,
    data: Vec<u8>,
    capacity: Option<Capacity>,
}

impl TypeIdContext {
    // create a typeid context,
    // if capacity is none, use mini required capacity.
    pub fn new(action: TypeIdAction, data: Vec<u8>, capacity: Option<Capacity>) -> Self {
        Self {
            action,
            data,
            capacity,
        }
    }
}

impl HandlerContext for TypeIdContext {}

impl TypeIdHandler {
    pub fn is_match(&self, script: &Script) -> bool {
        ScriptId::from(script).is_type_id()
    }

    fn create(
        &self,
        lock: &Script,
        viewer: &mut PrepareTransactionViewer,
        context: &TypeIdContext,
    ) -> Result<(), TxBuilderError> {
        let type_id_script = Script::new_builder()
            .code_hash(constants::TYPE_ID_CODE_HASH.pack())
            .hash_type(ScriptHashType::Type.into())
            .args(bytes::Bytes::from(vec![0u8; 32]).pack())
            .build();

        add_output(lock, type_id_script, context, viewer)
    }

    fn update(
        &self,
        input_point: &OutPoint,
        rpc_url: &str,
        lock: &Option<Script>,
        viewer: &mut PrepareTransactionViewer,
        context: &TypeIdContext,
    ) -> Result<(), TxBuilderError> {
        let header_dep_resolver = DefaultHeaderDepResolver::new(rpc_url);
        let tx_dep_provider = DefaultTransactionDependencyProvider::new(rpc_url, 10);

        let tx_hash = input_point.tx_hash();
        let deposit_header = header_dep_resolver
            .resolve_by_tx(&tx_hash)
            .map_err(TxBuilderError::Other)?
            .ok_or_else(|| TxBuilderError::ResolveHeaderDepByTxHashFailed(tx_hash.clone()))?;
        let (input_cell, input_data) = tx_dep_provider.get_cell_with_data(input_point)?;
        let type_id_script =
            input_cell
                .type_()
                .to_opt()
                .ok_or(TxBuilderError::InvalidParameter(anyhow!(
                    "the input cell has invalid type script"
                )))?;
        if !ScriptId::from(&type_id_script).is_type_id() {
            return Err(TxBuilderError::InvalidParameter(anyhow!(
                "the input cell does not have type id"
            )));
        }
        let input_lock = input_cell.lock();
        let lock = lock.as_ref().unwrap_or(&input_lock);
        add_output(lock, type_id_script, context, viewer)?;

        let live_cell = LiveCell {
            output: input_cell,
            output_data: input_data,
            out_point: input_point.clone(),
            block_number: deposit_header.number(),
            tx_index: u32::MAX, // TODO set correct tx_index
        };
        let transaction_input = TransactionInput::new(live_cell, 0);
        viewer.transaction_inputs.push(transaction_input);

        Ok(())
    }
}

fn add_output(
    lock: &Script,
    type_id_script: Script,
    context: &TypeIdContext,
    viewer: &mut PrepareTransactionViewer,
) -> Result<(), TxBuilderError> {
    let mut output = CellOutput::new_builder()
        .capacity(0u64.pack())
        .lock(lock.clone())
        .type_(Some(type_id_script).pack())
        .build();
    let occupied_capacity = output
        .occupied_capacity(Capacity::bytes(context.data.len()).unwrap())
        .unwrap()
        .as_u64();
    if let Some(capacity) = context.capacity {
        if capacity.as_u64() < occupied_capacity {
            return Err(TxBuilderError::InvalidCapacity(
                occupied_capacity,
                capacity.as_u64(),
            ));
        }
        output = output.as_builder().capacity(capacity.pack()).build();
    } else {
        output = output
            .as_builder()
            .capacity(occupied_capacity.pack())
            .build();
    }
    viewer.tx.output(output);
    viewer.tx.output_data(context.data.pack());
    Ok(())
}

// copy from https://github.com/nervosnetwork/ckb-cli/blob/develop/src/utils/other.rs#L325
pub fn calculate_type_id(first_cell_input: &CellInput, output_index: u64) -> [u8; 32] {
    let mut blake2b = new_blake2b();
    blake2b.update(first_cell_input.as_slice());
    blake2b.update(&output_index.to_le_bytes());
    let mut ret = [0u8; 32];
    blake2b.finalize(&mut ret);
    ret
}

impl ScriptHandler for TypeIdHandler {
    fn prepare_transaction(
        &self,
        viewer: &mut PrepareTransactionViewer,
        context: &mut dyn HandlerContext,
    ) -> Result<bool, TxBuilderError> {
        if let Some(args) = context.as_mut().downcast_mut::<TypeIdContext>() {
            match args.action {
                TypeIdAction::Handled => {
                    return Ok(true);
                }
                TypeIdAction::Create { ref lock } => {
                    self.create(lock, viewer, args)?;
                }
                TypeIdAction::Update {
                    ref input_point,
                    ref rpc_url,
                    ref lock,
                } => {
                    self.update(input_point, rpc_url, lock, viewer, args)?;
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn build_transaction(
        &self,
        tx_builder: &mut TransactionBuilder,
        script_group: &ScriptGroup,
        context: &dyn HandlerContext,
    ) -> Result<bool, TxBuilderError> {
        if !self.is_match(&script_group.script) {
            return Ok(false);
        }
        if let Some(_args) = context.as_any().downcast_ref::<TypeIdContext>() {
            let index = *script_group.output_indices.last().unwrap();
            let output = tx_builder.get_outputs()[index].clone();
            let type_ = output.type_().to_opt().unwrap();
            if type_.args().as_slice().len() != 32 {
                let type_ = type_
                    .as_builder()
                    .args(bytes::Bytes::from(vec![0u8; 32]).pack())
                    .build();
                let output = output.as_builder().type_(Some(type_).pack()).build();
                tx_builder.set_output(index, output);
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn init(&mut self, _network: &NetworkInfo) -> Result<(), TxBuilderError> {
        Ok(())
    }

    fn post_build(
        &self,
        index: usize,
        tx_builder: &mut TransactionBuilder,
        _context: &dyn HandlerContext,
    ) -> Result<bool, TxBuilderError> {
        if tx_builder.get_outputs().is_empty() {
            return Err(TxBuilderError::NoInput);
        }

        let output = tx_builder.get_outputs()[index].clone();
        let type_ = output.type_().to_opt().unwrap();
        if type_.args().as_slice() == [0u8; 32] {
            let type_ = type_
                .as_builder()
                .args(bytes::Bytes::from(vec![0u8; 32]).pack())
                .build();
            let output = output.as_builder().type_(Some(type_).pack()).build();
            tx_builder.set_output(index, output);
        }
        Ok(true)
    }
}
