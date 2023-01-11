use std::sync::Arc;

use crate::ws::{build_req, BuildConnection};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use digest::core_api::CoreWrapper;
use futures::{join, SinkExt, StreamExt};
use log::{error, warn};
use sha1::Sha1Core;
use tokio::{runtime::Runtime, task::JoinHandle};
use tokio_tungstenite::connect_async;
use tungstenite::http::Uri;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
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

pub enum NetworkError {}

#[derive(Debug)]
pub enum NetworkEvent {
    Connected(ConnectionHandle),
    Disconnected(ConnectionHandle),
    Message(ConnectionHandle, Vec<u8>),
    Error(Option<ConnectionHandle>, String),
}

pub type Sha1 = CoreWrapper<Sha1Core>;

pub struct Client {
    rt: Arc<Runtime>,
    handle: Option<JoinHandle<()>>,
    rx: Option<Arc<Receiver<NetworkEvent>>>,
    tx: Option<Arc<Sender<tungstenite::Message>>>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn is_running(&self) -> bool {
        self.handle.is_some() && self.rx.is_some() && self.tx.is_some()
    }

    pub fn new() -> Client {
        Client {
            rt: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("Could not build tokio runtime"),
            ),
            handle: None,
            rx: None,
            tx: None,
        }
    }

    pub fn connect(&mut self, endpoint: Url) {
        let url = BuildConnection::new(endpoint.as_str().parse::<Uri>().unwrap());
        let request = build_req(url).body(()).expect("Failed to build request");

        let (ev_tx, ev_rx) = unbounded();
        let (from_handler_tx, from_handler_rx) = unbounded();

        let event_loop = async move {
            let (ws_stream, _) = connect_async(request).await.expect("Failed to connect");
            let (mut write, read) = ws_stream.split();

            dbg!("connected");
            ev_tx
                .send(NetworkEvent::Connected(ConnectionHandle {
                    uuid: uuid::Uuid::nil(),
                }))
                .expect("failed to send network event");
            let read_handle = async move {
                read.for_each(|msg| async {
                    match msg {
                        Err(e) => {
                            error!("failed to receive message: {:?}", e);
                        }
                        Ok(tungstenite::Message::Binary(bts)) => {
                            ev_tx
                                .send(NetworkEvent::Message(
                                    ConnectionHandle {
                                        uuid: uuid::Uuid::nil(),
                                    },
                                    bts,
                                ))
                                .expect("failed to forward network message");
                        }
                        Ok(m) => {
                            warn!("unsupported message: {:?}", m);
                        }
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
                                warn!("failed to send message to server: {}", e);
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
    use super::*;
    use log::info;
    use std::str::FromStr;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_connect() {
        let room_url =
            "ws://127.0.0.1:3000/database/subscribe?name_or_address=extremeviolenceonspace";
        info!("connecting to spacetimedb server: {:?}", room_url);

        let mut client = Client::new();
        client.connect(Url::from_str(room_url).unwrap());

        sleep(Duration::from_secs(1));

        dbg!("Connected");
    }
}
