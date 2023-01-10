const PROTO_WEBSOCKET: &str = "websocket";

use hyper::Body;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use tokio_tungstenite::{
    tungstenite::protocol::{Role, WebSocketConfig},
    WebSocketStream,
};

use tungstenite::http::header::{
    CONNECTION, HOST, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_PROTOCOL,
    SEC_WEBSOCKET_VERSION, UPGRADE,
};
use tungstenite::http::{Response, StatusCode};

pub enum ErrorWs {
    Connect,
}

fn accept_key(key: &[u8]) -> String {
    const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::new();
    sha1.update(key);
    sha1.update(WS_GUID);
    let digest = sha1.finalize();
    base64::encode(digest)
}

pub fn accept_ws_res(
    key: &str,
    protocol: &str,
    custom_headers: HashMap<String, String>,
) -> Response<Body> {
    let mut builder = Response::builder()
        .header(UPGRADE, PROTO_WEBSOCKET)
        .header(CONNECTION, "upgrade")
        .header(SEC_WEBSOCKET_ACCEPT, accept_key(key.as_bytes()))
        .header(SEC_WEBSOCKET_PROTOCOL, protocol);

    for (k, v) in custom_headers {
        builder = builder.header(k, v);
    }

    builder
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .body(Body::empty())
        .unwrap()
}
