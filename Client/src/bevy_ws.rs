use bevy::prelude::*;
use spacetime_client_sdk::web_socket::{Client, ConnectionHandle, NetworkEvent};
use std::sync::Arc;

#[derive(Resource, Clone)]
pub(crate) struct WsClient {
    pub(crate) client: Arc<Client>,
    pub(crate) client_id: Option<ConnectionHandle>,
}

#[derive(Resource)]
pub(crate) struct WsMsg {
    pub(crate) ev: Vec<NetworkEvent>,
}

impl WsMsg {
    pub fn new() -> Self {
        Self { ev: Vec::new() }
    }
}

pub(crate) fn setup_net(mut commands: Commands) {
    let mut client = Client::new().expect("Fail to build ws client");
    client
        .connect(
            "ws://127.0.0.1:3000/database/subscribe?name_or_address=extremeviolenceonspace".into(),
        )
        .unwrap();

    commands.insert_resource(WsClient {
        client: Arc::new(client),
        client_id: None,
    });

    let network_events = WsMsg::new();
    commands.insert_resource(network_events);
}

pub(crate) fn consume_messages(ws: Res<WsClient>, mut network_events: ResMut<WsMsg>) {
    if !ws.client.is_running() {
        return;
    }
    while let Some(ev) = ws.client.try_recv() {
        dbg!("CM", &ev);
        network_events.ev.push(ev);
    }
}

pub(crate) fn handle_network_events(
    mut events: ResMut<WsMsg>,
    mut sink: EventWriter<NetworkEvent>,
) {
    for ev in events.ev.drain(..) {
        dbg!("HN", &ev);
        sink.send(ev);
    }
}

pub(crate) fn listen_for_events(mut evs: EventReader<NetworkEvent>) {
    for ev in evs.iter() {
        info!("received NetworkEvent : {:?}", ev);
    }
}
