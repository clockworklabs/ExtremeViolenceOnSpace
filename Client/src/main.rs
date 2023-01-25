//! A simplified implementation of the classic game (Extreme Violence)[http://www.geocities.ws/simesgreen/ev/index.html].
use crate::net::wait_for_players;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

use crate::sprites::{animate_sprite, ImageAssets};

mod components;
mod database;
mod input;
mod net;
mod player;
mod sprites;

use crate::components::*;
use crate::database::Player;
use crate::net::*;
use crate::player::*;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    AssetLoading,
    Matchmaking,
    InGame,
    Interlude,
}

pub const PLAYER_SIZE: (f64, f64) = (3121.0, 816.0);

const MAP_SIZE: i32 = 1024 * 2;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2dBundle::default(), MainCamera));

    commands.spawn(SpriteBundle {
        transform: Transform::from_scale(Vec3::new(1.5, 1.5, 0.0)),
        texture: asset_server.load("images/Background.png"),
        ..Default::default()
    });

    setup_net(commands);
}

fn reset_interlude_timer(mut timer: ResMut<InterludeTimer>) {
    timer.0 = 60;
}

fn interlude_timer(mut timer: ResMut<InterludeTimer>, mut state: ResMut<State<GameState>>) {
    if timer.0 == 0 {
        dbg!("interlude");
        state.set(GameState::InGame).unwrap();
    } else {
        timer.0 -= 1;
    }
}

fn main() {
    let mut app = App::new();

    app.add_state(GameState::AssetLoading)
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .with_collection::<ImageAssets>()
                .continue_to_state(GameState::Matchmaking),
        )
        .init_resource::<InterludeTimer>()
        .insert_resource(ClearColor(Color::rgb(0.53, 0.53, 0.53)))
        // .insert_resource(bevy::ecs::schedule::ReportExecutionOrderAmbiguities)
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
        .add_event::<Player>()
        .add_system_set(
            SystemSet::on_enter(GameState::Interlude).with_system(reset_interlude_timer),
        )
        .add_system_set(SystemSet::on_update(GameState::Interlude).with_system(interlude_timer))
        .add_system_set(SystemSet::on_enter(GameState::InGame).with_system(spawn_players))
        .add_system_set(
            SystemSet::on_update(GameState::InGame)
                // .with_system(animate_sprite)
                .with_system(move_players)
                .with_system(reload_bullet)
                .with_system(consume_messages)
                .with_system(handle_network_events)
                .with_system(listen_for_events)
                .with_system(fire_bullets.after(move_players).after(reload_bullet))
                .with_system(move_bullet)
                .with_system(kill_players.after(move_bullet).after(move_players)),
        )
        .add_system_set(SystemSet::on_enter(GameState::Matchmaking).with_system(setup))
        .add_system_set(SystemSet::on_update(GameState::Matchmaking).with_system(wait_for_players))
        .add_system_set(SystemSet::on_update(GameState::InGame).with_system(camera_follow))
        .add_system(bevy::window::close_on_esc)
        .run();
}
