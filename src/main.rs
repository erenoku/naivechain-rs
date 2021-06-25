use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use serde_json::{Result, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{thread, thread::JoinHandle};

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref BLOCK_CHAIN: Mutex<BlockChain> = Mutex::new(BlockChain {
        blocks: vec![BlockChain::get_genesis()],
    });
    static ref PEERS: Mutex<Vec<String>> = Mutex::new(vec![]);
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

    pub fn replace(&mut self, new_blocks: Vec<Block>) {
        self.blocks = new_blocks;
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

#[derive(Debug, Deserialize, Serialize, Clone)]
enum MessageType {
    QueryLatest,
    QueryAll,
    ResponseBlockchain,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Message {
    m_type: MessageType,
    content: String,
}

/// send snt to peer in different thread and return vector of its handler
/// use if gets response
fn send_to_peer(peer: String, msg: Message) -> JoinHandle<()> {
    thread::spawn(move || match TcpStream::connect(&peer) {
        Ok(mut stream) => {
            send_response(&mut stream, msg);
            let rsp = get_response(&mut stream);

            println!("{:?}", rsp);

            match rsp.m_type {
                MessageType::ResponseBlockchain => {
                    handle_blockchain_response(rsp);
                }
                _ => {}
            }
        }
        Err(e) => eprint!("Error Connecting to {}. {}", &peer, e),
    })
}

fn send_response(stream: &mut TcpStream, msg: Message) {
    let json = serde_json::to_string(&msg).unwrap();

    println!("{:?}", json);

    let j = json.as_bytes();
    let size = j.len() as u32;

    stream.write_u32::<BigEndian>(size).unwrap();
    stream.write_all(&j).unwrap();
}

fn handle_getting(mut stream: TcpStream) {
    let msg = get_response(&mut stream);

    println!("{:?}", msg);

    match msg.m_type {
        MessageType::QueryAll => {
            let msg = Message {
                m_type: MessageType::ResponseBlockchain,
                content: serde_json::to_string(&BLOCK_CHAIN.lock().unwrap().blocks).unwrap(),
            };
            send_response(&mut stream, msg);
        }
        MessageType::QueryLatest => {
            println!("latest");

            let msg = Message {
                m_type: MessageType::ResponseBlockchain,
                content: serde_json::to_string(&vec![BLOCK_CHAIN.lock().unwrap().get_latest()])
                    .unwrap(),
            };
            send_response(&mut stream, msg);
        }

        MessageType::ResponseBlockchain => {
            handle_blockchain_response(msg);
        }
    }
}

fn broadcast(msg: Message) {
    for peer in PEERS.lock().unwrap().iter() {
        send_to_peer(peer.clone(), msg.clone());
    }
}

fn handle_blockchain_response(res: Message) {
    let mut received_blocks: Vec<Block> = serde_json::from_str(&res.content).unwrap();
    received_blocks.sort_by(|a, b| a.index.cmp(&b.index));
    let latest_block_received = received_blocks.last().unwrap();
    let latest_block_held = BLOCK_CHAIN.lock().unwrap().get_latest();

    if latest_block_received.index > latest_block_held.index {
        println!(
            "blockchain possibly behind. We got: {}, Peer got: {}",
            latest_block_held.index, latest_block_received.index
        );

        if latest_block_held.hash == latest_block_received.previous_hash {
            println!("We can append the received block to our chain");
            BLOCK_CHAIN
                .lock()
                .unwrap()
                .blocks
                .push(latest_block_received.to_owned());
            // TODO: Broadcast
            broadcast(Message {
                m_type: MessageType::ResponseBlockchain,
                content: serde_json::to_string(&BLOCK_CHAIN.lock().unwrap().get_latest()).unwrap(),
            });
        } else if received_blocks.len() == 1 {
            println!("We have to query the chain from our peer");
            // TODO: Broadcast
            broadcast(Message {
                m_type: MessageType::QueryAll,
                content: String::new(),
            });
        } else {
            println!("Received blockchain is longer than current blockchain");
            BLOCK_CHAIN.lock().unwrap().replace(received_blocks);
        }
    } else {
        println!("received blockchain is not longer than current blockchain. Do nothing")
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
        PEERS.lock().unwrap().push(peer.to_owned());

        if peer == "" {
            break;
        }
        send_to_peer(
            peer.to_owned(),
            Message {
                m_type: MessageType::QueryLatest,
                // m_type: MessageType::QueryAll,
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
