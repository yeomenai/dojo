use std::collections::HashMap;

use anyhow::{ensure, Result};
use starknet::providers::jsonrpc::models::StateUpdate;
use starknet_api::{
    block::{
        Block, BlockBody, BlockHash, BlockHeader, BlockNumber, BlockStatus, BlockTimestamp,
        GasPrice,
    },
    core::{ContractAddress, GlobalRoot},
    hash::{pedersen_hash_array, StarkFelt},
    stark_felt,
    transaction::{Transaction, TransactionOutput},
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StarknetBlock {
    pub inner: Block,
    pub status: Option<BlockStatus>,
}

impl StarknetBlock {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        block_hash: BlockHash,
        parent_hash: BlockHash,
        block_number: BlockNumber,
        gas_price: GasPrice,
        state_root: GlobalRoot,
        sequencer: ContractAddress,
        timestamp: BlockTimestamp,
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        status: Option<BlockStatus>,
    ) -> Self {
        Self {
            inner: Block {
                header: BlockHeader {
                    block_hash,
                    parent_hash,
                    block_number,
                    gas_price,
                    state_root,
                    sequencer,
                    timestamp,
                },
                body: BlockBody {
                    transactions,
                    transaction_outputs,
                },
            },
            status,
        }
    }

    pub fn header(&self) -> &BlockHeader {
        &self.inner.header
    }

    pub fn body(&self) -> &BlockBody {
        &self.inner.body
    }

    pub fn insert_transaction(&mut self, transaction: Transaction) {
        self.inner.body.transactions.push(transaction);
    }

    pub fn transactions(&self) -> &[Transaction] {
        &self.inner.body.transactions
    }

    pub fn transaction_by_index(&self, transaction_id: usize) -> Option<Transaction> {
        self.inner.body.transactions.get(transaction_id).cloned()
    }

    pub fn block_hash(&self) -> BlockHash {
        self.inner.header.block_hash
    }

    pub fn block_number(&self) -> BlockNumber {
        self.inner.header.block_number
    }

    pub fn parent_hash(&self) -> BlockHash {
        self.inner.header.parent_hash
    }

    pub fn compute_block_hash(&self) -> BlockHash {
        BlockHash(pedersen_hash_array(&[
            stark_felt!(self.inner.header.block_number.0), // block number
            stark_felt!(0),                                // global_state_root
            self.inner.header.state_root.0,                // sequencer_address
            *self.inner.header.sequencer.0.key(),          // block_timestamp
            stark_felt!(self.inner.header.timestamp.0),    // transaction_count
            stark_felt!(self.inner.body.transactions.len() as u64), // transaction_commitment
            stark_felt!(0),                                // event_count
            stark_felt!(0),                                // event_commitment
            stark_felt!(0),                                // protocol_version
            stark_felt!(0),                                // extra_data
            stark_felt!(self.parent_hash().0),             // parent_block_hash
        ]))
    }
}

// TODO: add state archive
#[derive(Debug, Default)]
pub struct StarknetBlocks {
    pub hash_to_num: HashMap<BlockHash, BlockNumber>,
    pub num_to_block: HashMap<BlockNumber, StarknetBlock>,
    pub pending_block: Option<StarknetBlock>,
    pub num_to_state_update: HashMap<BlockNumber, StateUpdate>,
}

impl StarknetBlocks {
    pub fn append_block(&mut self, block: StarknetBlock) -> Result<()> {
        let block_number = block.block_number();
        let expected_block_number = BlockNumber(self.num_to_block.len() as u64);

        ensure!(
            expected_block_number == block_number,
            "unable to append block; expected block number {expected_block_number}, actual {block_number}"
        );

        self.hash_to_num.insert(block.block_hash(), block_number);
        self.num_to_block.insert(block_number, block);

        Ok(())
    }

    pub fn current_block_number(&self) -> BlockNumber {
        BlockNumber(self.total_blocks() as u64 - 1)
    }

    pub fn latest(&self) -> Option<StarknetBlock> {
        BlockNumber(self.num_to_block.len() as u64)
            .prev()
            .and_then(|num| self.num_to_block.get(&num).cloned())
    }

    pub fn by_hash(&self, block_hash: BlockHash) -> Option<StarknetBlock> {
        self.hash_to_num
            .get(&block_hash)
            .and_then(|block_number| self.by_number(*block_number))
    }

    pub fn by_number(&self, block_number: BlockNumber) -> Option<StarknetBlock> {
        self.num_to_block.get(&block_number).cloned()
    }

    pub fn transaction_by_block_num_and_index(
        &self,
        number: BlockNumber,
        index: usize,
    ) -> Option<Transaction> {
        self.num_to_block
            .get(&number)
            .and_then(|block| block.transaction_by_index(index))
    }

    pub fn total_blocks(&self) -> usize {
        self.num_to_block.len()
    }

    pub fn get_state_update(&self, block_number: BlockNumber) -> Option<StateUpdate> {
        self.num_to_state_update.get(&block_number).cloned()
    }
}
