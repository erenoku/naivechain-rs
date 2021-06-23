use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{thread, thread::JoinHandle};

#[derive(Debug, Clone)]
struct Block {
    index: u32,
    previous_hash: String,
    timestamp: u64,
    data: String,
    hash: String,
}

enum MessageType {
    QueryLatest,
    QueryAll,
    ResponseBlockchain,
}

impl Block {
    pub fn calculate_hash(&self) -> String {
        calculate_hash(
            &self.index,
            &self.previous_hash,
            &self.timestamp,
            &self.data,
        )
    }

    pub fn is_valid_next_block(prev: &Block, next: &Block) -> bool {
        if prev.index + 1 != prev.index {
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

struct BlockChain {
    blocks: Vec<Block>,
}

impl BlockChain {
    pub fn add(mut self, new: Block) {
        if Block::is_valid_next_block(&new, &self.get_latest()) {
            self.blocks.push(new)
        }
    }

    fn get_latest(&self) -> Block {
        self.blocks.last().unwrap().clone()
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

fn get_genesis_block() -> Block {
    Block {
        index: 0,
        previous_hash: String::from("0"),
        timestamp: 1465154705,
        data: String::from("my genesis block!!"),
        hash: String::from("816534932c2b7154836da6afc367695e6337db8a921823784c14378abed4f7d7"),
    }
}

#[derive(Deserialize, Debug, Serialize)]
struct Config {
    http_port: String,
    p2p_port: String,
    initial_peers: String,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            http_port: env::var("HTTP_PORT").unwrap_or_else(|_| String::from("8000")),
            p2p_port: env::var("P2P_PORT").unwrap_or_else(|_| String::from("5000")),
            initial_peers: env::var("INITIAL").unwrap_or_default(),
        }
    }
}

// connect to peers in different threads and return vector of their handlers
fn connect_to_peers(initial_peers: &'static [String]) -> Vec<JoinHandle<()>> {
    let mut handlers: Vec<JoinHandle<()>> = vec![];

    for peer in initial_peers {
        let handler = thread::spawn(move || match TcpStream::connect(peer) {
            Ok(mut stream) => {
                println!("Succesfully connected to {}", peer)
            }
            Err(e) => eprint!("Error Connecting to {}. {}", peer, e),
        });

        handlers.push(handler);
    }

    handlers
}

fn main() {
    let config = Config::from_env();
    println!("{:?}", config);

    let blockchain = Box::new(vec![get_genesis_block()]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash() {
        assert_eq!(
            calculate_hash(&0, "0", &1465154705, "my genesis block!!"),
            String::from("816534932c2b7154836da6afc367695e6337db8a921823784c14378abed4f7d7")
        )
    }
}
