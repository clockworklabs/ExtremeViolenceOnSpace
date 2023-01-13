use crate::errors::ClientError;
use crate::pub_sub::{Channel, Msg, PubSubDb};

use base64::{engine::general_purpose, Engine as _};
use crossbeam_channel::{unbounded, Receiver as CBReceiver, Sender as CBSender};
use hyper::http::request::Builder;
use sha1::{Digest, Sha1};
use tungstenite::http::header::{
    CONNECTION, HOST, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_PROTOCOL,
    SEC_WEBSOCKET_VERSION, UPGRADE,
};
use tungstenite::http::{Request, Uri};

use crate::client_api::{Message as ApiMessage, Message_oneof_type};
use futures::StreamExt;
use futures_util::SinkExt;
use protobuf::Message;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::{Message as WsMessage, Result};
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Error};
use url::Url;

const PROTO_WEBSOCKET: &str = "websocket";

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum Protocol {
    Text,
    Binary,
}

pub struct BuildConnection {
    protocol: Protocol,
    auth: Option<String>,
    url: Uri,
}

impl BuildConnection {
    pub fn new(url: Uri) -> Self {
        Self {
            protocol: Protocol::Binary,
            auth: None,
            url,
        }
    }
}

pub enum ErrorWs {
    Connect,
}

pub fn accept_key(key: &[u8]) -> String {
    const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::new();
    sha1.update(key);
    sha1.update(WS_GUID);
    let digest = sha1.finalize();
    general_purpose::STANDARD.encode(digest)
}

pub fn build_req(con: BuildConnection) -> Builder {
    let protocol = match con.protocol {
        Protocol::Text => "v1.text.spacetimedb",
        Protocol::Binary => "v1.bin.spacetimedb",
    };
    let key = tungstenite::handshake::client::generate_key();

    let b = Request::builder()
        .method("GET")
        .header(CONNECTION, "upgrade")
        .header(SEC_WEBSOCKET_PROTOCOL, protocol)
        .header(UPGRADE, PROTO_WEBSOCKET)
        .header(SEC_WEBSOCKET_VERSION, "13")
        .header(SEC_WEBSOCKET_ACCEPT, accept_key(key.as_bytes()))
        .header(SEC_WEBSOCKET_KEY, key);

    if let Some(host) = con.url.host() {
        b.header(HOST, host)
    } else {
        b
    }
    .uri(con.url)
}

fn process_read(
    pub_sub: &PubSubDb,
    msg: Option<Result<tungstenite::Message, tungstenite::Error>>,
) -> bool {
    println!("Received: {:?}", &msg);
    match msg {
        Some(Ok(msg)) => match msg {
            WsMessage::Text(txt) => {
                //pub_sub.publish_all(Msg::Op(txt));
                true
            }
            WsMessage::Binary(bin) => {
                let msg = ApiMessage::parse_from_bytes(&bin).unwrap();
                println!("Parsed: {:?}", &msg);

                if let Some(msg) = msg.field_type {
                    let state = pub_sub.state_lock();

                    match msg {
                        Message_oneof_type::functionCall(_) => {}
                        Message_oneof_type::subscriptionUpdate(_) => {}
                        Message_oneof_type::event(_) => {}
                        Message_oneof_type::transactionUpdate(_) => {}
                        Message_oneof_type::identityToken(token) => {
                            state
                                .client_to_game_sender
                                .send(Msg::Op(token.get_token().to_string()))
                                .unwrap();
                        }
                    }
                }
                true
            }
            WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Frame(_) => true,
            WsMessage::Close(_) => false,
        },
        Some(Err(err)) => {
            eprintln!("{}", err);
            false
        }
        _ => {
            println!("Gone");
            false
        }
    }
}

pub async fn tokio_setup(endpoint: &str, pub_sub: PubSubDb) -> Result<(), ClientError> {
    let url = BuildConnection::new(endpoint.parse::<Uri>()?);
    let request = build_req(url).body(())?;
    println!("Connecting to: {}", endpoint);
    let (ws_stream, res) = connect_async(request).await.unwrap();
    dbg!(res);
    println!("Listening on: {}", endpoint);
    let (mut write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            //Receive messages from the websocket
            msg = read.next() => {
                 if !process_read(&pub_sub,msg) {
                    break;
                }
            }
            // //Receive messages from the game
            // game_msg = pub_sub. game_to_client_receiver.recv() => {
            //     let game_msg = game_msg.unwrap();
            //     ws_sender.send(Message::Text(game_msg)).await?;
            // }
        }
    }

    println!("Finished");

    Ok(())
}

async fn accept_connection(
    peer: SocketAddr,
    stream: TcpStream,
    pub_sub: PubSubDb,
    client_to_game_sender: CBSender<String>,
) {
    if let Err(e) = handle_connection(peer, stream, pub_sub, client_to_game_sender).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => println!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(
    peer: SocketAddr,
    stream: TcpStream,
    pub_sub: PubSubDb,
    client_to_game_sender: CBSender<String>,
) -> Result<()> {
    println!("New WebSocket connection: {}", peer);

    let ws_stream = accept_async(stream).await?;

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    //Create a tokio sync channel to for messages from the game to each client
    let (game_to_client_sender, mut game_to_client_receiver) = tokio::sync::mpsc::channel(100);

    //Get the number of clients for a client id
    let num_clients = pub_sub.len() as u32;
    //Store the incremented client id and the game to client sender in the clients hashmap
    //pub_sub.subscribe(Channel::new(num_clients + 1));

    //This loop uses the tokio select! macro to receive messages from either the websocket receiver
    //or the game to client receiver
    loop {
        tokio::select! {
            //Receive messages from the websocket
            msg = ws_receiver.next() => {
                match msg {
                    Some(msg) => {
                        let msg = msg?;
                        if msg.is_text() ||msg.is_binary() {
                            client_to_game_sender.send(msg.to_string()).map_err(|_err| {
                                tungstenite::Error::ConnectionClosed
                                //ClientError::Other(err.into())
                            })?;
                        } else if msg.is_close() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            //Receive messages from the game
            game_msg = game_to_client_receiver.recv() => {
                let game_msg = game_msg.unwrap();
                ws_sender.send(WsMessage::Text(game_msg)).await?;
            }

        }
    }

    Ok(())
}
