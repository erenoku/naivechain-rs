use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;

use crate::{Block, BLOCK_CHAIN, PEERS};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum MessageType {
    QueryLatest,
    QueryAll,
    ResponseBlockchain,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub m_type: MessageType,
    pub content: String,
}

impl Message {
    /// send self to the peer and handle the response
    /// if doesn't handle response use send_response(&self, stream: &mut TcpStream)
    pub fn send_to_peer(&self, peer: String) {
        match TcpStream::connect(&peer) {
            Ok(mut stream) => {
                // send_response(&mut stream, msg);
                self.send_response(&mut stream);
                let rsp = Message::get_response(&mut stream);

                println!("{:?}", rsp);

                if let MessageType::ResponseBlockchain = rsp.m_type {
                    rsp.handle_blockchain_response();
                }
            }
            Err(e) => eprint!("Error Connecting to {}. {}", &peer, e),
        }
    }

    pub fn send_response(&self, stream: &mut TcpStream) {
        let json = serde_json::to_string(&self).unwrap();

        println!("{:?}", json);

        let j = json.as_bytes();
        let size = j.len() as u32;

        stream.write_u32::<BigEndian>(size).unwrap();
        stream.write_all(j).unwrap();
    }

    pub fn get_response(stream: &mut TcpStream) -> Message {
        let size = stream.read_u32::<BigEndian>().unwrap(); // read the size sended from client

        let mut buffer = vec![0_u8; size as usize]; // create a buffer with that size
        stream.read_exact(&mut buffer).unwrap(); // read to buffer

        let string_buffer = String::from_utf8(buffer).unwrap(); // convert to string
        serde_json::from_str(&string_buffer).unwrap()
    }

    pub fn handle_blockchain_response(&self) {
        let mut received_blocks: Vec<Block> = serde_json::from_str(&self.content).unwrap();
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

                Message {
                    m_type: MessageType::ResponseBlockchain,
                    content: serde_json::to_string(&BLOCK_CHAIN.lock().unwrap().get_latest())
                        .unwrap(),
                }
                .broadcast();
            } else if received_blocks.len() == 1 {
                println!("We have to query the chain from our peer");

                Message {
                    m_type: MessageType::QueryAll,
                    content: String::new(),
                }
                .broadcast();
            } else {
                println!("Received blockchain is longer than current blockchain");
                BLOCK_CHAIN.lock().unwrap().replace(received_blocks);
            }
        } else {
            println!("received blockchain is not longer than current blockchain. Do nothing")
        }
    }

    fn broadcast(self) {
        for peer in PEERS.lock().unwrap().iter() {
            self.send_to_peer(peer.clone());
        }
    }
}
