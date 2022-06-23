#[macro_use]
extern crate lazy_static;

use actix_web::http::StatusCode;
use actix_web::{get, web, App, FromRequest, HttpResponse, HttpServer};
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
    static ref SELF_ADDR: Mutex<String> = Mutex::new(String::new());
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
            http_port: env::var("HTTP_PORT").unwrap_or_else(|_| "8000".into()),
            p2p_port: env::var("P2P_PORT").unwrap_or_else(|_| "5000".into()),
            initial_peers: env::var("INITIAL").unwrap_or_default(),
        }
    }
}

fn handle_getting(mut stream: TcpStream) {
    let msg = Message::get_response(&mut stream);

    // println!("{:?}", msg);

    match msg.m_type {
        MessageType::QueryAll => {
            let msg = Message {
                m_type: MessageType::ResponseBlockchain,
                content: serde_json::to_string(&BLOCK_CHAIN.lock().unwrap().blocks).unwrap(),
            };
            println!("query all sending message {:?}", msg);
            msg.send_request(&mut stream);
        }
        MessageType::QueryLatest => {
            let msg = Message {
                m_type: MessageType::ResponseBlockchain,
                content: serde_json::to_string(&vec![BLOCK_CHAIN.lock().unwrap().get_latest()])
                    .unwrap(),
            };

            println!("query latest sending {:?}", msg);

            // println!("latest query: {}", msg.content);

            // send_response(&mut stream, msg);
            msg.send_request(&mut stream);
        }
        MessageType::ResponseBlockchain => {
            // println!("response");
            msg.handle_blockchain_response();
        }
        MessageType::Greet => {
            msg.get_new_peers();

            let new_msg = Message {
                m_type: MessageType::GreetBack,
                content: serde_json::to_string(&PEERS.lock().unwrap().clone()).unwrap(),
            };

            new_msg.send_request(&mut stream);
        }
        _ => {}
    }
}

// init the p2p server return the thread handler
fn init_p2p_server(port: String) -> JoinHandle<()> {
    thread::spawn(move || {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();

        // accept connections and process them, spawning a new thread for each one
        // println!("Server listening on port {}", port);
        for stream in listener.incoming() {
            thread::spawn(move || handle_getting(stream.unwrap())); // TODO: unwrap
        }
    })
}

fn greet_peer(peer: &str) {
    let mut content = PEERS.lock().unwrap().clone();
    content.push(SELF_ADDR.lock().unwrap().clone());

    let msg = Message {
        m_type: MessageType::Greet,
        content: serde_json::to_string(&content).unwrap(),
    };
    msg.send_to_peer(peer.to_string());
}

fn main() {
    let config = Config::from_env();
    // println!("{:?}", config);

    *SELF_ADDR.lock().unwrap() = format!("localhost:{}", config.p2p_port);

    for peer in config.initial_peers.split(',') {
        if peer.is_empty() {
            break;
        }

        // TODO: greet in different thread and not just use localhost as ip
        greet_peer(peer);

        PEERS.lock().unwrap().push(peer.to_owned());

        Message {
            m_type: MessageType::QueryLatest,
            content: String::new(),
        }
        .send_to_peer(peer.to_owned());
    }

    let http_port = config.http_port.clone(); // will go inside move closure
    let http_handler = thread::spawn(move || init_http_server(http_port).unwrap());
    init_p2p_server(config.p2p_port);

    http_handler.join().unwrap();
}

#[get("/blocks")]
async fn blocks() -> actix_web::Result<HttpResponse> {
    // response
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/json")
        .body(serde_json::to_string(&BLOCK_CHAIN.lock().unwrap().blocks).unwrap()))
}

#[get("/peers")]
async fn peers() -> actix_web::Result<HttpResponse> {
    // println!("peers {}", PEERS.lock().unwrap().join("\n"));
    Ok(HttpResponse::build(StatusCode::OK).body(PEERS.lock().unwrap().join("\n")))
}

async fn connect_to_peer(peer: String) -> actix_web::Result<HttpResponse> {
    if peer.is_empty() {
        return Ok(HttpResponse::build(StatusCode::BAD_REQUEST).body(""));
    }

    PEERS.lock().unwrap().push(peer.clone());
    // println!("{}", PEERS.lock().unwrap().join(""));

    Message {
        m_type: MessageType::QueryLatest,
        content: String::new(),
    }
    .send_to_peer(peer);

    Ok(HttpResponse::build(StatusCode::OK).body(""))
}

async fn mine_block(body: String) -> actix_web::Result<HttpResponse> {
    let next_block = Block::generate_next(body, &BLOCK_CHAIN.lock().unwrap());

    // println!("{:?}", next_block);

    BLOCK_CHAIN.lock().unwrap().add(next_block);

    let msg = Message {
        m_type: MessageType::ResponseBlockchain,
        content: serde_json::to_string(&vec![BLOCK_CHAIN.lock().unwrap().get_latest()]).unwrap(),
    };

    println!("mine_block broadcasting {:?}", msg);

    msg.broadcast();

    Ok(HttpResponse::build(StatusCode::OK).body(""))
}

#[actix_web::main]
async fn init_http_server(http_port: String) -> std::io::Result<()> {
    // println!("start http server on port {}", http_port);

    HttpServer::new(|| {
        App::new()
            .service(blocks)
            .service(
                web::resource("/addPeer")
                    .route(web::post().to(connect_to_peer))
                    .data(String::configure(|cfg| cfg.limit(4096))),
            )
            .service(
                web::resource("/mineBlock")
                    .route(web::post().to(mine_block))
                    .data(String::configure(|cfg| cfg.limit(4096))),
            )
            .service(peers)
    })
    .bind(format!("127.0.0.1:{}", http_port))?
    .run()
    .await
}
