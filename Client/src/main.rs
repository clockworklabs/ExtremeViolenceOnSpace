//! A simplified implementation of the classic game "Breakout".
mod bevy_ws;
mod components;
mod database;
mod input;
mod player;
mod sprites;

use std::collections::HashMap;

use crate::bevy_ws::*;
use crate::components::*;
use crate::database::*;
use crate::input::*;
use crate::player::*;

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_ggrs::{GGRSPlugin, Session};
use ggrs::{Message, NonBlockingSocket, PlayerType};

use crate::sprites::{animate_sprite, ImageAssets};
use spacetime_client_sdk::messages::SpaceDbResponse;
use spacetime_client_sdk::web_socket::NetworkEvent;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    AssetLoading,
    Matchmaking,
    InGame,
}

pub const PLAYER_SIZE: (f64, f64) = (3121.0, 816.0);

const MAP_SIZE: i32 = 300;

struct GgrsConfig;

impl ggrs::Config for GgrsConfig {
    // 4-directions + fire fits easily in a single byte
    type Input = u8;
    type State = u8;
    // Matchbox' WebRtcSocket addresses are strings
    type Address = String;
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2dBundle::default(), MainCamera));

    commands.spawn(SpriteBundle {
        transform: Transform::from_scale(Vec3::new(1.5, 1.5, 0.0)),
        texture: asset_server.load("images/Background.png"),
        ..Default::default()
    });

    setup_net(commands);
}

struct MsgRec {
    socket: WsClient,
}

impl NonBlockingSocket<String> for MsgRec {
    fn send_to(&mut self, msg: &Message, addr: &String) {}

    fn receive_all_messages(&mut self) -> Vec<(String, Message)> {
        let mut received_messages = Vec::new();

        loop {
            match self.socket.client.try_recv() {
                None => return received_messages,
                Some(msg) => match msg {
                    NetworkEvent::Connected(_) => continue,
                    NetworkEvent::Disconnected(_) => return received_messages,
                    NetworkEvent::Message(handle, msg) => {}
                    NetworkEvent::Error(handle, err) => {
                        panic!("{:?} on {:?}", err, &handle)
                    }
                },
            }
        }

        received_messages
        //vec![]
    }
}

fn wait_for_players(
    mut commands: Commands,
    mut socket: ResMut<WsClient>,
    mut state: ResMut<State<GameState>>,
) {
    if !socket.client.is_running() {
        return;
    }
    let mut clients = HashMap::with_capacity(2);

    for msg in socket.client.try_recv() {
        match msg {
            NetworkEvent::Connected(client_id) => {
                socket.client_id = Some(client_id.clone());
                create_new_player(&socket.client, PlayerId::One, &client_id);
                create_new_player(&socket.client, PlayerId::Two, &client_id);
            }
            NetworkEvent::Message(ref client_id, msg) => {
                warn!("Get {msg:?}");
                match msg {
                    SpaceDbResponse::SubscriptionUpdate(table) => {
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
                    _ => {}
                }
            }
            NetworkEvent::Disconnected(_) => {
                socket.client_id = None;
                return;
            }
            NetworkEvent::Error(client_id, _) => return,
        };
    }
    dbg!(&clients);
    let num_players = 2;
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

    // create a GGRS P2P session
    let mut session_builder = ggrs::SessionBuilder::<GgrsConfig>::new()
        .with_num_players(num_players)
        .with_input_delay(2);

    for (player_handle, player_rec) in clients {
        dbg!(player_handle);
        if player_handle == PlayerId::One {
            commands.insert_resource(LocalPlayerHandle(player_handle));
        }
        session_builder = session_builder
            .add_player(
                PlayerType::Remote(player_rec.uuid.to_string()),
                player_handle as usize,
            )
            .expect("failed to add player");
    }

    // move the socket out of the resource (required because GGRS takes ownership of it)
    let socket = socket.clone();

    // start the GGRS session
    let session = session_builder
        .start_p2p_session(MsgRec { socket })
        .expect("failed to start session");

    commands.insert_resource(Session::P2PSession(session));
    info!("All peers have joined, going in-game");
    state.set(GameState::InGame).unwrap();
}

fn main() {
    let mut app = App::new();

    GGRSPlugin::<GgrsConfig>::new()
        .with_input_system(input)
        .with_rollback_schedule(
            Schedule::default().with_stage(
                "ROLLBACK_STAGE",
                SystemStage::single_threaded()
                    .with_system_set(
                        SystemSet::on_enter(GameState::InGame).with_system(spawn_players),
                    )
                    .with_system_set(
                        SystemSet::on_update(GameState::InGame)
                            .with_system(move_players.label(Systems::Move))
                            .with_system(animate_sprite),
                    ),
            ),
        )
        .register_rollback_component::<Transform>()
        .build(&mut app);

    app.add_state(GameState::AssetLoading)
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .with_collection::<ImageAssets>()
                .continue_to_state(GameState::Matchmaking),
        )
        .insert_resource(ClearColor(Color::rgb(0.53, 0.53, 0.53)))
        .insert_resource(Msaa { samples: 1 })
        // .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
        //     TIMESTEP_5_PER_SECOND,
        // )))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "SpacetimeDB Game".into(),
                        fit_canvas_to_parent: true,
                        ..default()
                    },
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_event::<NetworkEvent>()
        .add_system_set(
            SystemSet::on_enter(GameState::Matchmaking)
                .with_system(setup)
                .with_system(wait_for_players),
        )
        .add_system_set(SystemSet::on_enter(GameState::InGame).with_system(spawn_players))
        .add_system_set(SystemSet::on_update(GameState::InGame).with_system(camera_follow))
        .add_system(bevy::window::close_on_esc)
        .run();
}
