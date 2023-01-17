use std::str::FromStr;
use std::sync::Arc;

use crate::errors::ClientError;
use crate::messages::{process_msg, serialize_msg, SpaceDbRequest, SpaceDbResponse};
use crate::ws::{build_req, BuildConnection};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use futures::{join, SinkExt, StreamExt};
use log::{error, info, warn};
use tokio::{runtime::Runtime, task::JoinHandle};
use tokio_tungstenite::connect_async;
use tungstenite::http::Uri;
use uuid::Uuid;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ConnectionHandle {
    pub uuid: Uuid,
}

impl ConnectionHandle {
    pub fn new() -> ConnectionHandle {
        ConnectionHandle {
            uuid: Uuid::new_v4(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.uuid
    }
}

#[derive(Debug)]
pub enum NetworkEvent {
    Connected(ConnectionHandle),
    Disconnected(ConnectionHandle),
    Message(ConnectionHandle, SpaceDbResponse),
    Error(Option<ConnectionHandle>, ClientError),
}

pub struct Client {
    rt: Arc<Runtime>,
    handle: Option<JoinHandle<()>>,
    rx: Option<Arc<Receiver<NetworkEvent>>>,
    tx: Option<Arc<Sender<tungstenite::Message>>>,
    con: BuildConnection,
}

impl Client {
    pub fn new(endpoint: String) -> Result<Self, ClientError> {
        let url = Uri::from_str(&endpoint)?;
        let con = BuildConnection::new(url);

        Ok(Client {
            rt: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()?,
            ),
            handle: None,
            rx: None,
            tx: None,
            con,
        })
    }

    pub fn is_running(&self) -> bool {
        self.handle.is_some() && self.rx.is_some() && self.tx.is_some()
    }

    pub fn connect(&mut self) -> Result<(), ClientError> {
        let url = self.con.url.clone();
        info!("Connecting to: {}...", &url);
        let request = build_req(&self.con).body(())?;
        let (ev_tx, ev_rx) = unbounded();
        let (from_handler_tx, from_handler_rx) = unbounded();

        let event_loop = async move {
            let (ws_stream, response) = match connect_async(request).await {
                Ok(x) => x,
                Err(err) => {
                    ev_tx
                        .send(NetworkEvent::Error(None, ClientError::Tungstenite(err)))
                        .expect("failed to send error network event");
                    return;
                }
            };
            info!("Connected to: {} DONE", url);
            if let Some(token) = response.headers().get("spacetime-identity-token") {
                if let Ok(token) = token.to_str() {
                    self.con.auth = Some(token.to_string());
                }
            }

            let (mut write, read) = ws_stream.split();
            ev_tx
                .send(NetworkEvent::Connected(ConnectionHandle::new()))
                .expect("failed to send network event");

            let read_handle = async move {
                read.for_each(|msg| async {
                    if let Some(msg) = process_msg(msg) {
                        ev_tx.send(msg).expect("failed to forward network message");
                    }
                })
                .await;
            };

            let write_handle = async move {
                loop {
                    let req = from_handler_rx.try_recv();

                    match req {
                        Err(TryRecvError::Empty) => {
                            // TODO: REPLACE SPINLOCK !
                            continue;
                        }
                        Err(e) => {
                            warn!("failed to forward message to sink: {}", e);
                        }
                        Ok(ev) => {
                            if let Err(e) = write.send(ev).await {
                                error!("failed to send message to server: {}", e);
                            }
                        }
                    }
                }
            };
            join!(read_handle, write_handle);
        };
        self.handle = Some(self.rt.spawn(event_loop));
        self.rx = Some(Arc::new(ev_rx));
        self.tx = Some(Arc::new(from_handler_tx));

        Ok(())
    }

    pub fn try_recv(&self) -> Option<NetworkEvent> {
        if let Some(channel) = &self.rx {
            match channel.try_recv() {
                Err(TryRecvError::Empty) => None,
                Err(e) => {
                    warn!("failed to forward message to sink: {}", e);
                    None
                }
                Ok(ev) => Some(ev),
            }
        } else {
            warn!("trying to receive message with an uninitialized client");
            None
        }
    }

    pub fn send_message(&self, msg: SpaceDbRequest) {
        if let Some(msg) = serialize_msg(&self.con, msg) {
            self.send_raw_message(msg);
        }
    }

    pub fn send_raw_message(&self, msg: tokio_tungstenite::tungstenite::Message) {
        if let Some(channel) = &self.tx {
            if let Err(e) = channel.send(msg) {
                warn!("failed to forward message, sink: {:?}", e);
            }
        } else {
            warn!("trying to send message with an uninitialized client",);
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::{sleep, Duration};

    use super::*;

    #[test]
    fn test_connect() {
        let room_url =
            "ws://127.0.0.1:3000/database/subscribe?name_or_address=extremeviolenceonspace";
        info!("connecting to spacetimedb server: {:?}", room_url);

        let mut client = Client::new().unwrap();
        client.connect(room_url.into()).unwrap();

        dbg!("Connected");
        dbg!(client.is_running());

        client.send_raw_message(Message::Text("Hi".to_string()));

        let _ = client
            .rt
            .block_on(async { sleep(Duration::from_millis(100)).await });
        for msg in client.try_recv() {
            dbg!(msg);
        }
    }
}
