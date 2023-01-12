//! A simplified implementation of the classic game "Breakout".
mod bevy_ws;
mod components;
mod input;

use crate::bevy_ws::{message_system, setup_net, ClientMSG};
use crate::components::*;
use crate::input::*;
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_ggrs::*;
use ggrs::{Message, NonBlockingSocket, PlayerType};

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

#[derive(Copy, Clone, PartialEq, Eq)]
enum PlayerId {
    One,
    Two,
}

#[derive(Component, PartialEq, Eq)]
struct Player {
    handle: PlayerId,
}

impl Player {
    pub fn as_idx(&self) -> usize {
        match self.handle {
            PlayerId::One => 0,
            PlayerId::Two => 1,
        }
    }
}

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

fn start_matchbox_socket(mut commands: Commands) {
    // var url = new Uri($"ws://{host}/database/subscribe?name_or_address={nameOrAddress}");
    //
    // let room_url =
    //     "ws://127.0.0.1:3000/database/subscribe?name_or_address=extreme_violence_spacetimedb";
    // info!("connecting to spacetimedb server: {:?}", room_url);
    //
    // let mut client = Client::new();
    // client.connect(Url::from_str(room_url).unwrap());

    // let (socket, message_loop) = WebRtcSocket::new(room_url);
    //
    // The message loop needs to be awaited, or nothing will happen.
    // We do this here using bevy's task system.
    //IoTaskPool::get().spawn(client.send_message()).detach();
    //
    //commands.insert_resource(Some(client));
    // commands.insert_resource(client);
}
//
// pub struct QuinnetServerPlugin {}
//
// impl Default for QuinnetServerPlugin {
//     fn default() -> Self {
//         Self {}
//     }
// }
//
// fn create_server(mut commands: Commands, runtime: Res<Client>) {
//     let room_url = "ws://127.0.0.1:3000/database/subscribe?name_or_address=";
//     info!("connecting to spacetimedb server: {:?}", room_url);
//
//     let mut client = Client::new();
//     client.connect(Url::from_str(room_url).unwrap());
//
//     commands.insert_resource(client);
// }
//
// impl Plugin for QuinnetServerPlugin {
//     fn build(&self, app: &mut App) {
//         app.add_startup_system_to_stage(StartupStage::PreStartup, create_server)
//         //    .add_system_to_stage(CoreStage::PreUpdate, update_sync_server)
//         ;
//
//         if app.world.get_resource_mut::<Client>().is_none() {
//             app.insert_resource(Client::new());
//         }
//     }
// }

fn setup(mut commands: Commands) {
    let camera_bundle = Camera2dBundle::default();
    //camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn(camera_bundle);

    setup_net(commands)
}

struct MsgRec {}

impl NonBlockingSocket<String> for MsgRec {
    fn send_to(&mut self, msg: &Message, addr: &String) {}

    fn receive_all_messages(&mut self) -> Vec<(String, Message)> {
        vec![]
    }
}

fn wait_for_players(
    mut commands: Commands,
    mut socket: ResMut<ClientMSG>,
    mut state: ResMut<State<GameState>>,
) {
    let socket = socket.as_mut();

    // Check for new connections
    let clients = socket.pub_sub.state_lock();

    let num_players = 2;
    if clients.clients.len() < num_players {
        return; // wait for more players
    }

    info!("All peers have joined, going in-game");
    // create a GGRS P2P session
    let mut session_builder = ggrs::SessionBuilder::<GgrsConfig>::new()
        .with_num_players(num_players)
        .with_input_delay(2);

    for (player_handle, player) in clients.clients.iter().take(num_players) {
        let player_handle = player_handle.identity - 1;
        dbg!(player_handle);
        session_builder = session_builder
            .add_player(
                PlayerType::Remote(player_handle.to_string()),
                player_handle as usize,
            )
            .expect("failed to add player");
    }

    // move the socket out of the resource (required because GGRS takes ownership of it)
    let socket = socket.client_to_game_receiver.clone();

    // start the GGRS session
    let session = session_builder
        .start_p2p_session(MsgRec {})
        //.start_synctest_session()
        .expect("failed to start session");

    commands.insert_resource(Session::P2PSession(session));

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

    // GGRSPlugin::<GgrsConfig>::new()
    //     .with_input_system(input)
    //     .with_rollback_schedule(Schedule::default().with_stage(
    //         "ROLLBACK_STAGE",
    //         SystemStage::single_threaded().with_system(move_players),
    //     ))
    //     .register_rollback_component::<Transform>()
    //     .build(&mut app);

    app.add_state(GameState::AssetLoading)
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .with_collection::<ImageAssets>()
                .continue_to_state(GameState::Matchmaking),
        )
        .insert_resource(ClearColor(Color::rgb(0.53, 0.53, 0.53)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "SpacetimeDB Game".into(),
                fit_canvas_to_parent: true,
                ..default()
            },
            ..default()
        }))
        .add_system_set(
            SystemSet::on_enter(GameState::Matchmaking)
                .with_system(start_matchbox_socket)
                .with_system(setup),
        )
        .add_system_set(SystemSet::on_update(GameState::Matchmaking).with_system(wait_for_players))
        .add_system_set(SystemSet::on_enter(GameState::InGame).with_system(message_system))
        .add_system_set(SystemSet::on_enter(GameState::InGame).with_system(spawn_players))
        .add_system_set(SystemSet::on_update(GameState::InGame).with_system(camera_follow))
        .add_system(bevy::window::close_on_esc)
        .run();

    // app.add_plugins(DefaultPlugins.set(WindowPlugin {
    //     window: WindowDescriptor {
    //         title: "SpacetimeDB Game".into(),
    //         fit_canvas_to_parent: true,
    //         ..default()
    //     },
    //     ..default()
    // }))
    // .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
    //     TIMESTEP_5_PER_SECOND,
    // )))
    // .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.20)))
    // .add_startup_system(setup)
    // .add_startup_system(spawn_players)
    // // .add_system(wait_for_players)
    // .add_system(message_system)
    // //.add_startup_system(start_matchbox_socket)
    // .add_system(bevy::window::close_on_esc)
    // .run();
}
