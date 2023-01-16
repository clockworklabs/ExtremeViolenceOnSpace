use bevy::prelude::*;
use spacetime_client_sdk::web_socket::{Client, ConnectionHandle, NetworkEvent};
use std::sync::Arc;
use tokio::sync::Mutex;

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

#[derive(Default, Debug)]
pub struct WebSocketClient {}

impl Plugin for WebSocketClient {
    fn build(&self, app: &mut App) {
        let mut client = Client::new().expect("Fail to build ws client");
        client
            .connect(
                "ws://127.0.0.1:3000/database/subscribe?name_or_address=extremeviolenceonspace"
                    .into(),
            )
            .unwrap();
        //        let router = Arc::new(Mutex::new(GenericParser::new()));
        let network_events = WsMsg::new();
        //         app.add_startup_system_to_stage(StartupStage::PreStartup, create_server)
        //         //    .add_system_to_stage(CoreStage::PreUpdate, update_sync_server)

        app.insert_resource(WsClient {
            client: Arc::new(client),
            client_id: None,
        })
        // .insert_resource(router)
        .insert_resource(network_events)
        .add_event::<NetworkEvent>()
        .add_stage_before(CoreStage::First, "network", SystemStage::single_threaded())
        .add_system_to_stage("network", consume_messages)
        .add_system_to_stage("network", handle_network_events);
    }
}

fn consume_messages(ws: Res<WsClient>, mut network_events: ResMut<WsMsg>) {
    if !ws.client.is_running() {
        return;
    }
    while let Some(ev) = ws.client.try_recv() {
        network_events.ev.push(ev);
    }
}

pub(crate) fn handle_network_events(
    mut events: ResMut<WsMsg>,
    mut sink: EventWriter<NetworkEvent>,
) {
    for ev in events.ev.drain(..) {
        sink.send(ev);
    }
}

pub(crate) fn listen_for_events(mut evs: EventReader<NetworkEvent>) {
    for ev in evs.iter() {
        info!("received NetworkEvent : {:?}", ev);
    }
}
