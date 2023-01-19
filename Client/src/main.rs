//! A simplified implementation of the classic game "Breakout".
mod bevy_ws;
mod components;
mod database;
mod input;
use std::collections::HashMap;
use std::time::Duration;

use crate::bevy_ws::{
    consume_messages, handle_network_events, listen_for_events, setup_net, WsClient,
};
use crate::components::*;
use crate::database::*;
use crate::input::*;
use bevy::app::ScheduleRunnerSettings;
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_ggrs::*;
use ggrs::{Message, NonBlockingSocket, PlayerType};
use spacetime_client_sdk::messages::SpaceDbResponse;
use spacetime_client_sdk::web_socket::NetworkEvent;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    AssetLoading,
    Matchmaking,
    InGame,
}

#[derive(AssetCollection, Resource)]
struct ImageAssets {
    #[asset(path = "images/Alien.png")]
    alien: Handle<Image>,
    #[asset(path = "images/CowBoy.png")]
    cowboy: Handle<Image>,
}

const MAP_SIZE: i32 = 41;

struct GgrsConfig;

impl ggrs::Config for GgrsConfig {
    // 4-directions + fire fits easily in a single byte
    type Input = u8;
    type State = u8;
    // Matchbox' WebRtcSocket addresses are strings
    type Address = String;
}

fn spawn_players(mut commands: Commands, asset_server: Res<AssetServer>) {
    //, mut rip: ResMut<RollbackIdProvider>
    commands.spawn(SpriteBundle {
        transform: Transform::from_scale(Vec3::new(1.5, 1.5, 0.0)),
        texture: asset_server.load("images/Background.png"),
        ..Default::default()
    });

    let size = 300.;

    // Player 1
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(-200., 0., 100.)),
            sprite: Sprite {
                custom_size: Some(Vec2::new(size, size)),
                ..default()
            },
            texture: asset_server.load("images/CowBoy.png"),
            ..default()
        })
        .insert(Player {
            handle: PlayerId::One,
        })
        .insert(BulletReady(true))
        .insert(MoveDir(-Vec2::X))
    //    .insert(Rollback::new(rip.next_id()))
    ;

    // Player 2
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(200., 0., 100.)),
            sprite: Sprite {
                custom_size: Some(Vec2::new(size, size)),
                ..default()
            },
            texture: asset_server.load("images/Alien.png"),
            ..default()
        })
        .insert(Player {
            handle: PlayerId::Two,
        })
        .insert(BulletReady(true))
        .insert(MoveDir(Vec2::X))
    //    .insert(Rollback::new(rip.next_id()))
    ;
}

fn move_players(
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut player_query: Query<(&mut Transform, &mut MoveDir, &Player)>,
) {
    for (mut transform, mut move_direction, player) in player_query.iter_mut() {
        let (input, _) = inputs[player.as_idx()];
        let direction = direction(input);

        if direction == Vec2::ZERO {
            continue;
        }

        move_direction.0 = direction;

        let move_speed = 0.13;
        let move_delta = direction * move_speed;

        let old_pos = transform.translation.xy();
        let limit = Vec2::splat(MAP_SIZE as f32 / 2. - 0.5);
        let new_pos = (old_pos + move_delta).clamp(-limit, limit);

        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y;
    }
}

fn setup(mut commands: Commands) {
    let camera_bundle = Camera2dBundle::default();
    //camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn(camera_bundle);

    setup_net(commands);
    //setup_net2(commands);
}

struct MsgRec {
    socket: WsClient,
}

impl NonBlockingSocket<String> for MsgRec {
    fn send_to(&mut self, msg: &Message, addr: &String) {}

    fn receive_all_messages(&mut self) -> Vec<(String, Message)> {
        //self.socket.iter().map(|msg| )
        vec![]
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
            //info!("Waiting for Player2");
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

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
struct LocalPlayerHandle(usize);

fn camera_follow(
    player_handle: Option<Res<LocalPlayerHandle>>,
    player_query: Query<(&Player, &Transform)>,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    let player_handle = match player_handle {
        Some(handle) => handle.0,
        None => return, // Session hasn't started yet
    };

    for (player, player_transform) in player_query.iter() {
        if player.as_idx() != player_handle {
            continue;
        }

        let pos = player_transform.translation;

        for mut transform in camera_query.iter_mut() {
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }
    }
}

const TIMESTEP_5_PER_SECOND: f64 = 30.0 / 60.0;

fn main() {
    let mut app = App::new();

    GGRSPlugin::<GgrsConfig>::new()
        .with_input_system(input)
        .with_rollback_schedule(Schedule::default().with_stage(
            "ROLLBACK_STAGE",
            SystemStage::single_threaded().with_system(move_players),
        ))
        .register_rollback_component::<Transform>()
        .build(&mut app);

    app.add_state(GameState::AssetLoading)
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .with_collection::<ImageAssets>()
                .continue_to_state(GameState::Matchmaking),
        )
        .insert_resource(ClearColor(Color::rgb(0.53, 0.53, 0.53)))
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            TIMESTEP_5_PER_SECOND,
        )))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "SpacetimeDB Game".into(),
                fit_canvas_to_parent: true,
                ..default()
            },
            ..default()
        }))
        .add_event::<NetworkEvent>()
        .add_system_set(SystemSet::on_enter(GameState::Matchmaking).with_system(setup))
        // .add_system_set(
        //     SystemSet::on_update(GameState::Matchmaking).with_system(handle_network_events),
        // )
        .add_system_set(SystemSet::on_update(GameState::Matchmaking).with_system(wait_for_players))
        // .add_system_set(SystemSet::on_update(GameState::Matchmaking).with_system(consume_messages))
        // .add_system_set(SystemSet::on_update(GameState::Matchmaking).with_system(listen_for_events))
        .add_system_set(SystemSet::on_enter(GameState::InGame).with_system(spawn_players))
        .add_system_set(SystemSet::on_update(GameState::InGame).with_system(camera_follow))
        .add_system(bevy::window::close_on_esc)
        .run();
}
