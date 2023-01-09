//! A simplified implementation of the classic game "Breakout".

use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::tasks::IoTaskPool;
use bevy_ggrs::*;
use extreme_violence_spacetimedb_client::web_socket::Client;
use ggrs::InputStatus;
use std::str::FromStr;
use url::Url;

enum PlayerId {
    One,
    Two,
}

#[derive(Component)]
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

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;
const INPUT_FIRE: u8 = 1 << 4;

fn spawn_players(mut commands: Commands, mut rip: ResMut<RollbackIdProvider>) {
    // Player 1
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(-2., 0., 0.)),
            sprite: Sprite {
                color: Color::rgb(0., 0.47, 1.),
                custom_size: Some(Vec2::new(1., 1.)),
                ..default()
            },
            ..default()
        })
        .insert(Player {
            handle: PlayerId::One,
        })
        .insert(Rollback::new(rip.next_id()));

    // Player 2
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(2., 0., 0.)),
            sprite: Sprite {
                color: Color::rgb(1.57, 0.37, 0.66),
                custom_size: Some(Vec2::new(1., 1.)),
                ..default()
            },
            ..default()
        })
        .insert(Player {
            handle: PlayerId::Two,
        })
        .insert(Rollback::new(rip.next_id()));
}

fn input(_: In<ggrs::PlayerHandle>, keys: Res<Input<KeyCode>>) -> u8 {
    let mut input = 0u8;

    if keys.any_pressed([KeyCode::Up, KeyCode::W]) {
        input |= INPUT_UP;
    }
    if keys.any_pressed([KeyCode::Down, KeyCode::S]) {
        input |= INPUT_DOWN;
    }
    if keys.any_pressed([KeyCode::Left, KeyCode::A]) {
        input |= INPUT_LEFT
    }
    if keys.any_pressed([KeyCode::Right, KeyCode::D]) {
        input |= INPUT_RIGHT;
    }
    if keys.any_pressed([KeyCode::Space, KeyCode::Return]) {
        input |= INPUT_FIRE;
    }

    input
}

fn move_players(
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut player_query: Query<(&mut Transform, &Player)>,
) {
    for (mut transform, player) in player_query.iter_mut() {
        let (input, _) = inputs[player.as_idx()];

        let mut direction = Vec2::ZERO;

        if input & INPUT_UP != 0 {
            direction.y += 1.;
        }
        if input & INPUT_DOWN != 0 {
            direction.y -= 1.;
        }
        if input & INPUT_RIGHT != 0 {
            direction.x += 1.;
        }
        if input & INPUT_LEFT != 0 {
            direction.x -= 1.;
        }
        if direction == Vec2::ZERO {
            continue;
        }

        let move_speed = 0.13;
        let move_delta = (direction * move_speed).extend(0.);

        transform.translation += move_delta;
    }
}

fn start_matchbox_socket(mut commands: Commands) {
    // var url = new Uri($"ws://{host}/database/subscribe?name_or_address={nameOrAddress}");

    let room_url =
        "ws://127.0.0.1:3000/database/subscribe?name_or_address=extreme_violence_spacetimedb";
    info!("connecting to spacetimedb server: {:?}", room_url);

    let mut client = Client::new();
    client.connect(Url::from_str(room_url).unwrap());

    // let (socket, message_loop) = WebRtcSocket::new(room_url);
    //
    // The message loop needs to be awaited, or nothing will happen.
    // We do this here using bevy's task system.
    //IoTaskPool::get().spawn(client.send_message()).detach();
    //
    //commands.insert_resource(Some(client));
    commands.insert_resource(client);
}

pub struct QuinnetServerPlugin {}

impl Default for QuinnetServerPlugin {
    fn default() -> Self {
        Self {}
    }
}

fn create_server(mut commands: Commands, runtime: Res<Client>) {
    let room_url = "ws://127.0.0.1:3000/database/subscribe?name_or_address=";
    info!("connecting to spacetimedb server: {:?}", room_url);

    let mut client = Client::new();
    client.connect(Url::from_str(room_url).unwrap());

    commands.insert_resource(client);
}

impl Plugin for QuinnetServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system_to_stage(StartupStage::PreStartup, create_server)
        //    .add_system_to_stage(CoreStage::PreUpdate, update_sync_server)
        ;

        if app.world.get_resource_mut::<Client>().is_none() {
            app.insert_resource(Client::new());
        }
    }
}

fn setup(mut commands: Commands) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn_bundle(camera_bundle);
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "SpacetimeDB Game".into(),
                fit_canvas_to_parent: true,
                ..default()
            },
            ..default()
        }))
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.20)))
        .add_startup_system(setup)
        .add_startup_system(spawn_players)
        .add_system(move_players)
        .add_startup_system(start_matchbox_socket)
        .run();
}
