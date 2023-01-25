use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use spacetime_client_sdk::messages::SpaceDbResponse;
use spacetime_client_sdk::web_socket::{Client, ConnectionHandle, NetworkEvent};

use crate::components::{InterludeTimer, LocalPlayerHandle};
use crate::database::*;
use crate::player::current_player;
use crate::GameState;

#[derive(Resource, Clone)]
pub(crate) struct WsClient {
    pub(crate) client: Arc<Client>,
    pub(crate) client_id: Option<ConnectionHandle>,
}

#[derive(Resource)]
pub(crate) struct WsMsg {
    pub(crate) ev: Vec<Player>,
}

impl WsMsg {
    pub fn new() -> Self {
        Self { ev: Vec::new() }
    }
}

pub(crate) fn setup_net(mut commands: Commands) {
    let mut client =
        Client::new("127.0.0.1:3000", "extremeviolenceonspace").expect("Fail to build ws client");
    client.connect().expect("Fail to connect to SpaceTimeDb");

    commands.insert_resource(WsClient {
        client: Arc::new(client),
        client_id: None,
    });

    let network_events = WsMsg::new();
    commands.insert_resource(network_events);
}

#[derive(Resource)]
struct MsgRec {
    socket: WsClient,
}

pub(crate) fn wait_for_players(
    mut commands: Commands,
    mut socket: ResMut<WsClient>,
    mut state: ResMut<State<GameState>>,
    mut interlude_timer: ResMut<InterludeTimer>,
) {
    if !socket.client.is_running() {
        return;
    }
    let mut clients = HashMap::with_capacity(2);

    if let Some(msg) = socket.client.try_recv() {
        match msg {
            NetworkEvent::Connected(client_id) => {
                socket.client_id = Some(client_id.clone());
                create_new_player(&socket.client, PlayerId::One, &client_id);
                create_new_player(&socket.client, PlayerId::Two, &client_id);
            }
            NetworkEvent::Message(ref client_id, msg) => {
                warn!("Get {msg:?}");
                if let SpaceDbResponse::SubscriptionUpdate(table) = msg {
                    for x in table.table_updates {
                        if x.table_name == "PlayerComponent" {
                            info!("Inserting players...");
                            for row in x.table_row_operations {
                                let player_id = *row.row[0].as_i8().unwrap();
                                let player = match player_id {
                                    0 => PlayerId::One,
                                    1 => PlayerId::Two,
                                    x => panic!("Invalid PlayerId {x}"),
                                };
                                info!("Row player {:?} {:?}", &player, &row);
                                clients.insert(player, client_id.clone());
                            }
                        }
                    }
                }
            }
            NetworkEvent::Disconnected(_) => {
                socket.client_id = None;
                return;
            }
            NetworkEvent::Error(client_id, err) => {
                panic!("Get a error from the server for {client_id:?}: {err}");
            }
        };
    }
    dbg!(&clients);
    match clients.len() {
        0 => {
            info!("Waiting for Player1");
            return;
        }
        1 => {
            info!("Waiting for Player2");
            return;
        }
        2 => {
            info!("Players ready!");
        }
        _ => panic!("This is a 2 player-only game!"),
    }

    let current_player = current_player();
    for (player_handle, _player_rec) in clients {
        dbg!(player_handle);
        if player_handle == current_player {
            commands.insert_resource(LocalPlayerHandle(player_handle));
        }
    }

    // move the socket out of the resource (required because GGRS takes ownership of it)
    let socket = socket.clone();
    commands.insert_resource(MsgRec { socket });
    info!("All peers have joined, going in-game");
    interlude_timer.0 = 3 * 60;
    state.set(GameState::Interlude).unwrap();
}

pub(crate) fn consume_messages(ws: Res<WsClient>, mut player_query: Query<&mut Player>) {
    if !ws.client.is_running() {
        return;
    }
    while let Some(msg) = ws.client.try_recv() {
        match msg {
            NetworkEvent::Connected(_) => {}
            NetworkEvent::Message(_, msg) => {
                warn!("CONSUME {msg:?}");
                if let SpaceDbResponse::TransactionUpdate(table) = msg {
                    for x in table.subscription_update.table_updates {
                        if x.table_name == "PlayerComponent" {
                            for row in x.table_row_operations {
                                let player_id = *row.row[0].as_i8().unwrap();
                                let input = *row.row[2].as_i8().unwrap();
                                let player = match player_id {
                                    0 => PlayerId::One,
                                    1 => PlayerId::Two,
                                    x => panic!("Invalid PlayerId {x}"),
                                };
                                info!("Move player {:?}: {input} {:?}", &player, &row);
                                for mut p in player_query.iter_mut() {
                                    if p.handle == player {
                                        p.input = input as u8;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            NetworkEvent::Disconnected(_) => {}
            NetworkEvent::Error(client_id, err) => {
                panic!("Get a error from the server for {client_id:?}: {err}");
            }
        };
    }
}

pub(crate) fn handle_network_events(mut events: ResMut<WsMsg>, mut sink: EventWriter<Player>) {
    for ev in events.ev.drain(..) {
        dbg!("HN", &ev);
        sink.send(ev);
    }
}

pub(crate) fn listen_for_events(mut evs: EventReader<Player>) {
    for ev in evs.iter() {
        info!("received Player : {:?}", ev);
    }
}
