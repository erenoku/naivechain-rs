#[macro_use]
extern crate lazy_static;

use actix_web::http::StatusCode;
use actix_web::{get, web, App, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::env;
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::{thread, thread::JoinHandle};

mod block;
mod chain;
mod message;

use crate::block::Block;
use crate::chain::BlockChain;
use crate::message::{Message, MessageType};

// global static varibales
lazy_static! {
    static ref BLOCK_CHAIN: Mutex<BlockChain> = Mutex::new(BlockChain {
        blocks: vec![BlockChain::get_genesis()],
    });
    static ref PEERS: Mutex<Vec<String>> = Mutex::new(vec![]);
}

#[derive(Deserialize, Debug, Serialize, Clone)]
struct Config {
    http_port: String,
    p2p_port: String,
    initial_peers: String,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            http_port: env::var("HTTP_PORT").unwrap_or("8000".into()),
            p2p_port: env::var("P2P_PORT").unwrap_or("5000".into()),
            initial_peers: env::var("INITIAL").unwrap_or_default(),
        }
    }
}

fn handle_getting(mut stream: TcpStream) {
    let msg = Message::get_response(&mut stream);

    println!("{:?}", msg);

    match msg.m_type {
        MessageType::QueryAll => {
            let msg = Message {
                m_type: MessageType::ResponseBlockchain,
                content: serde_json::to_string(&BLOCK_CHAIN.lock().unwrap().blocks).unwrap(),
            };
            msg.send_response(&mut stream);
        }
        MessageType::QueryLatest => {
            println!("latest");

            let msg = Message {
                m_type: MessageType::ResponseBlockchain,
                content: serde_json::to_string(&vec![BLOCK_CHAIN.lock().unwrap().get_latest()])
                    .unwrap(),
            };
            // send_response(&mut stream, msg);
            msg.send_response(&mut stream);
        }

        MessageType::ResponseBlockchain => {
            println!("response");
            msg.handle_blockchain_response();
        }
    }
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

    for peer in config.initial_peers.split(',') {
        PEERS.lock().unwrap().push(peer.to_owned());

        if peer.is_empty() {
            break;
        }
        Message {
            m_type: MessageType::QueryLatest,
            // m_type: MessageType::QueryAll,
            content: String::new(),
        }
        .send_to_peer(peer.to_owned());
    }

    let http_port = config.http_port.clone(); // will go inside move closure
    let http_handler = thread::spawn(move || init_http_server(http_port).unwrap());
    let p2p_handler = init_p2p_server(config.p2p_port);

    http_handler.join().unwrap();
    // p2p_handler.join().unwrap();
}

#[get("/blocks")]
async fn blocks(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    // response
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/json")
        .body(serde_json::to_string(&BLOCK_CHAIN.lock().unwrap().blocks).unwrap()))
}

async fn mine_block(req: HttpRequest, body: String) -> actix_web::Result<HttpResponse> {
    let next_block = Block::generate_next(body, &BLOCK_CHAIN.lock().unwrap());

    println!("{:?}", next_block);

    BLOCK_CHAIN.lock().unwrap().add(next_block);

    Message {
        m_type: MessageType::ResponseBlockchain,
        content: serde_json::to_string(&vec![BLOCK_CHAIN.lock().unwrap().get_latest()]).unwrap(),
    }
    .broadcast();

    Ok(HttpResponse::build(StatusCode::OK).body("jknşj"))
}

#[actix_web::main]
async fn init_http_server(http_port: String) -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(blocks).service(
            web::resource("/mineBlock")
                .route(web::post().to(mine_block))
                .data(String::configure(|cfg| cfg.limit(4096))),
        )
    })
    .bind(format!("127.0.0.1:{}", http_port))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_hash() {
    //     assert_eq!(
    //         calculate_hash(&0, "0", &1465154705, "my genesis block!!"),
    //         String::from("816534932c2b7154836da6afc367695e6337db8a921823784c14378abed4f7d7")
    //     )
    // }
}
