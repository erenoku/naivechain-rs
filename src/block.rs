use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::chain::BlockChain;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Block {
    pub index: u32,
    pub previous_hash: String,
    pub timestamp: u64,
    pub data: String,
    pub hash: String,
}

impl Block {
    /// calculate hash of the whole block
    pub fn calculate_hash(&self) -> String {
        calculate_hash(
            &self.index,
            &self.previous_hash,
            &self.timestamp,
            &self.data,
        )
    }

    /// check if the next block is valid for the given previous block
    pub fn is_valid_next_block(next: &Block, prev: &Block) -> bool {
        if prev.index + 1 != next.index {
            return false;
        }
        if prev.hash != next.previous_hash {
            return false;
        }
        if next.calculate_hash() != next.hash {
            return false;
        }

        true
    }

    /// generate the next block with given block_data
    pub fn generate_next(block_data: String, chain: &BlockChain) -> Block {
        let prev_block = chain.get_latest();
        let next_index = prev_block.index + 1;
        let next_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let next_hash = calculate_hash(&next_index, &prev_block.hash, &next_timestamp, &block_data);

        Block {
            index: next_index,
            previous_hash: prev_block.hash,
            timestamp: next_timestamp,
            data: block_data,
            hash: next_hash,
        }
    }
}

fn calculate_hash(index: &u32, previous_hash: &str, timestamp: &u64, data: &str) -> String {
    let mut hasher = Sha256::new();

    hasher.update(index.to_string().as_str());
    hasher.update(previous_hash);
    hasher.update(timestamp.to_string().as_str());
    hasher.update(data);

    format!("{:x}", hasher.finalize())
}
