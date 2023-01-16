use base64::{engine::general_purpose, Engine as _};
use hyper::http::request::Builder;
use sha1::{Digest, Sha1};
use tungstenite::http::header::{
    CONNECTION, HOST, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_PROTOCOL,
    SEC_WEBSOCKET_VERSION, UPGRADE,
};
use tungstenite::http::{Request, Uri};

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
