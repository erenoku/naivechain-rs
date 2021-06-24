use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use serde_json::{Result, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{thread, thread::JoinHandle};

#[macro_use]
extern crate lazy_static;

// lazy_static! (
// )

lazy_static! {
    static ref BLOCK_CHAIN: BlockChain = BlockChain {
        blocks: vec![BlockChain::get_genesis()],
    };
}

#[derive(Serialize, Debug, Clone, PartialEq)]
struct Block {
    index: u32,
    previous_hash: String,
    timestamp: u64,
    data: String,
    hash: String,
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

    fn get_genesis() -> Block {
        Block {
            index: 0,
            previous_hash: String::from("0"),
            timestamp: 1465154705,
            data: String::from("my genesis block!!"),
            hash: String::from("816534932c2b7154836da6afc367695e6337db8a921823784c14378abed4f7d7"),
        }
    }

    fn is_valid(&self) -> bool {
        if *self.blocks.first().unwrap() != BlockChain::get_genesis() {
            return false;
        }

        let mut temp_blocks = vec![self.blocks.first().unwrap()];

        for i in 1..self.blocks.len() {
            if Block::is_valid_next_block(
                temp_blocks.get(i - 1).unwrap(),
                self.blocks.get(i).unwrap(),
            ) {
                temp_blocks.push(self.blocks.get(i).unwrap());
            } else {
                return false;
            }
        }

        true
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

#[derive(Debug, Deserialize, Serialize)]
enum MessageType {
    QueryLatest,
    QueryAll,
    ResponseBlockchain,
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    m_type: MessageType,
    content: String,
}

// connect to peers in different threads and return vector of their handlers
fn send_to_peer(peer: String, msg: Message) {
    thread::spawn(move || match TcpStream::connect(&peer) {
        Ok(mut stream) => {
            send_response(&mut stream, msg);
            let rcp = get_response(&mut stream);

            println!("{:?}", rcp);
        }
        Err(e) => eprint!("Error Connecting to {}. {}", &peer, e),
    });
}

fn send_response(stream: &mut TcpStream, msg: Message) {
    let json = serde_json::to_string(&msg).unwrap();
    let j = json.as_bytes();
    let size = j.len() as u32;

    stream.write_u32::<BigEndian>(size).unwrap();
    stream.write_all(&j).unwrap();
}

fn handle_getting(mut stream: TcpStream) {
    let msg = get_response(&mut stream);

    match msg.m_type {
        MessageType::QueryAll => {
            println!("query all")
        }
        MessageType::QueryLatest => {
            let msg = Message {
                m_type: MessageType::ResponseBlockchain,
                content: serde_json::to_string(&BLOCK_CHAIN.get_latest()).unwrap(),
            };
            let json = serde_json::to_string(&msg).unwrap();
            let msg = json.as_bytes();
            let size = msg.len() as u32;

            stream.write_u32::<BigEndian>(size).unwrap();
            stream.write_all(&msg).unwrap();
        }
        MessageType::ResponseBlockchain => {}
    }
}

fn get_response(stream: &mut TcpStream) -> Message {
    let size = stream.read_u32::<BigEndian>().unwrap(); // read the size sended from client

    let mut buffer = vec![0_u8; size as usize]; // create a buffer with that size
    stream.read_exact(&mut buffer).unwrap(); // read to buffer

    let string_buffer = String::from_utf8(buffer).unwrap(); // convert to string
    serde_json::from_str(&string_buffer).unwrap()
}

// init the p2p server return the thread handler
fn init_p2p_server(port: String) -> JoinHandle<()> {
    thread::spawn(move || {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();

        // accept connections and process them, spawning a new thread for each one
        println!("Server listening on port {}", port);
        for stream in listener.incoming() {
            thread::spawn(move || handle_getting(stream.unwrap())); // TODO: unwrap
        }
    })
}

fn main() {
    let config = Config::from_env();
    println!("{:?}", config);

    for peer in config.initial_peers.split(",") {
        if peer == "" {
            break;
        }
        send_to_peer(
            peer.to_owned(),
            Message {
                m_type: MessageType::QueryLatest,
                content: String::new(),
            },
        );
    }

    let p2p_handler = init_p2p_server(config.p2p_port);
    p2p_handler.join().unwrap();
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
