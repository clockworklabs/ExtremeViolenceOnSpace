//! A simplified implementation of the classic game "Breakout".

use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::tasks::IoTaskPool;
use bevy_ggrs::*;

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

enum PlayerId {
    One,
    Two,
}

#[derive(Component)]
struct Player {
    handle: PlayerId,
}

fn spawn_players(mut commands: Commands) {
    // Player 1
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0., 0.47, 1.),
                custom_size: Some(Vec2::new(1., 1.)),
                ..default()
            },
            ..default()
        })
        .insert(Player {
            handle: PlayerId::One,
        });

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
        });
}

fn move_players(keys: Res<Input<KeyCode>>, mut player_query: Query<&mut Transform, With<Player>>) {
    let mut direction = Vec2::ZERO;

    if keys.any_pressed([KeyCode::Up, KeyCode::W]) {
        direction.y += 1.;
    }
    if keys.any_pressed([KeyCode::Down, KeyCode::S]) {
        direction.y -= 1.;
    }
    if keys.any_pressed([KeyCode::Right, KeyCode::D]) {
        direction.x += 1.;
    }
    if keys.any_pressed([KeyCode::Left, KeyCode::A]) {
        direction.x -= 1.;
    }
    if direction == Vec2::ZERO {
        return;
    }

    let move_speed = 0.13;
    let move_delta = (direction * move_speed).extend(0.);

    for mut transform in player_query.iter_mut() {
        transform.translation += move_delta;
    }
}

fn start_matchbox_socket(mut commands: Commands) {
    // let room_url = "ws://127.0.0.1:3536/extreme_bevy?next=2";
    // info!("connecting to matchbox server: {:?}", room_url);
    // let (socket, message_loop) = WebRtcSocket::new(room_url);
    //
    // // The message loop needs to be awaited, or nothing will happen.
    // // We do this here using bevy's task system.
    // IoTaskPool::get().spawn(message_loop).detach();
    //
    // commands.insert_resource(Some(socket));
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
        .run();
}
