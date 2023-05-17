use std::any::Any;

use crate::{
    core::TransactionBuilder, tx_builder::TxBuilderError, unlock::MultisigConfig, NetworkInfo,
    ScriptGroup,
};

use self::sighash::Secp256k1Blake160SighashAllScriptContext;

use super::builder::PrepareTransactionViewer;

pub mod dao;
pub mod multisig;
pub mod sighash;
pub mod typeid;

pub trait ScriptHandler {
    fn prepare_transaction(
        &self,
        _viewer: &mut PrepareTransactionViewer,
        _context: &mut dyn HandlerContext,
    ) -> Result<bool, TxBuilderError> {
        Ok(false)
    }

    /// Try to build transaction with the given script_group and context.
    ///
    /// Return true if script_group and context are matched, otherwise return false.
    fn build_transaction(
        &self,
        tx_builder: &mut TransactionBuilder,
        script_group: &ScriptGroup,
        context: &dyn HandlerContext,
    ) -> Result<bool, TxBuilderError>;

    fn post_build(
        &self,
        _index: usize,
        _tx_builder: &mut TransactionBuilder,
        _context: &dyn HandlerContext,
    ) -> Result<bool, TxBuilderError> {
        Ok(false)
    }
    fn init(&mut self, network: &NetworkInfo) -> Result<(), TxBuilderError>;
}

pub trait Type2Any: 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> Type2Any for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait HandlerContext: Type2Any {}

pub struct HandlerContexts {
    pub contexts: Vec<Box<dyn HandlerContext>>,
}

impl Default for HandlerContexts {
    fn default() -> Self {
        Self {
            contexts: vec![Box::new(Secp256k1Blake160SighashAllScriptContext {})],
        }
    }
}

impl HandlerContexts {
    pub fn new(context: Box<dyn HandlerContext>) -> Self {
        Self {
            contexts: vec![context],
        }
    }
    pub fn new_sighash() -> Self {
        Self {
            contexts: vec![Box::new(Secp256k1Blake160SighashAllScriptContext {})],
        }
    }

    pub fn new_multisig(config: MultisigConfig) -> Self {
        Self {
            contexts: vec![Box::new(
                multisig::Secp256k1Blake160MultisigAllScriptContext::new(config),
            )],
        }
    }

    pub fn new_dao_withdraw_phrase1() -> Self {
        Self {
            contexts: vec![Box::<dao::WithdrawPhrase1Context>::default()],
        }
    }

    pub fn new_dao_withdraw_phrase2() -> Self {
        Self {
            contexts: vec![Box::<dao::WithdrawPhrase1Context>::default()],
        }
    }

    pub fn add_context(&mut self, context: Box<dyn HandlerContext>) {
        self.contexts.push(context);
    }

    /// extend this context with other contexts
    pub fn extend_contexts(&mut self, contexts: HandlerContexts) {
        self.contexts.extend(contexts.contexts);
    }
}
