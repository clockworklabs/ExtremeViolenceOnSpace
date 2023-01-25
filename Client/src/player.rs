use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use std::env;

use crate::components::*;
use crate::database::*;
use crate::input::{direction, fire, input};
use crate::net::WsClient;
use crate::sprites::{AnimationTimer, ImageAssets, SpritesheetAnimator};
use crate::{GameState, MAP_SIZE};

pub(crate) fn current_player() -> PlayerId {
    env::args()
        .skip(1)
        .map(|x| {
            if x == "two" {
                PlayerId::Two
            } else {
                PlayerId::One
            }
        })
        .next()
        .unwrap_or(PlayerId::One)
}

fn spawn_player(commands: &mut Commands, asset: &Res<ImageAssets>, player: PlayerId) {
    let (img, pos, move_dir) = match player {
        PlayerId::One => (&asset.cowboy, Vec3::new(200., 0., 100.), Vec2::X),
        PlayerId::Two => (&asset.alien, Vec3::new(-200., 0., 100.), -Vec2::X),
    };
    let player_animations = SpritesheetAnimator::new(player);

    //draw single texture from sprite sheet starting at index 0
    commands
        .spawn(SpriteSheetBundle {
            transform: Transform {
                translation: pos,
                ..Default::default()
            },
            sprite: TextureAtlasSprite {
                custom_size: Some(Vec2::new(300., 300.)),
                index: 0,
                ..default()
            },
            texture_atlas: img.clone(),
            ..Default::default()
        })
        .insert(AnimationTimer(Timer::from_seconds(
            0.1,
            TimerMode::Repeating,
        )))
        .insert(Player::new(player))
        .insert(player_animations)
        .insert(BulletReady(true))
        .insert(MoveDir(move_dir));
}

pub(crate) fn spawn_players(
    mut commands: Commands,
    asset_server: Res<ImageAssets>,
    player_query: Query<Entity, With<Player>>,
    bullet_query: Query<Entity, With<Bullet>>,
) {
    for player in player_query.iter() {
        commands.entity(player).despawn_recursive();
    }
    for bullet in bullet_query.iter() {
        commands.entity(bullet).despawn_recursive();
    }

    dbg!("spawning players");
    spawn_player(&mut commands, &asset_server, PlayerId::One);
    spawn_player(&mut commands, &asset_server, PlayerId::Two);
}

pub(crate) fn move_players(
    local_player: Option<Res<LocalPlayerHandle>>,
    keys: Res<Input<KeyCode>>,
    socket: ResMut<WsClient>,
    mut player_query: Query<(
        &mut SpritesheetAnimator,
        &mut TextureAtlasSprite,
        &mut Transform,
        &mut MoveDir,
        &mut Player,
    )>,
) {
    let local_player = if let Some(x) = local_player {
        x
    } else {
        // Session hasn't started yet;
        return;
    };

    for (mut animator, mut sprite, mut transform, mut move_direction, mut player) in
        player_query.iter_mut()
    {
        player.input = if player.handle == local_player.0 {
            let input = input(&keys);
            move_player(&socket.client, player.handle, input);
            input
        } else {
            player.input
        };

        let (direction, animation) = direction(animator.animation, player.input);
        animator.set_state(animation, &mut sprite);

        if direction == Vec2::ZERO {
            continue;
        }
        //dbg!(animation);

        move_direction.0 = direction;

        let move_speed = 20.13;
        let move_delta = direction * move_speed;

        let old_pos = transform.translation.xy();
        let limit = Vec2::splat(MAP_SIZE as f32 / 2. - 0.5);
        let new_pos = (old_pos + move_delta).clamp(-limit, limit);

        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y;
    }
}

pub(crate) fn reload_bullet(mut query: Query<(&mut BulletReady, &Player)>) {
    for (mut can_fire, player) in query.iter_mut() {
        if !fire(player.input) {
            can_fire.0 = true;
        }
    }
}

pub(crate) fn fire_bullets(
    mut commands: Commands,
    images: Res<ImageAssets>,
    mut player_query: Query<(&Transform, &Player, &mut BulletReady, &MoveDir)>,
) {
    for (transform, player, mut bullet_ready, move_dir) in player_query.iter_mut() {
        //dbg!(fire(player.input), bullet_ready.0);
        if fire(player.input) && bullet_ready.0 {
            let player_pos = transform.translation.xy();
            let pos = player_pos + move_dir.0 * PLAYER_RADIUS + BULLET_RADIUS;
            let bullet = if player.handle == PlayerId::One {
                images.bullet_cowboy.clone()
            } else {
                images.bullet_alien.clone()
            };

            commands.spawn((
                Bullet,
                *move_dir,
                SpriteBundle {
                    transform: Transform::from_translation(pos.extend(200.))
                        .with_rotation(Quat::from_rotation_arc_2d(Vec2::X, move_dir.0)),
                    texture: bullet,
                    sprite: Sprite {
                        //Making the bullets smaller
                        custom_size: Some(Vec2::new(1920.0 / 20.0, 1080.0 / 20.0)),
                        ..default()
                    },
                    ..default()
                },
            ));
            bullet_ready.0 = false;
        }
    }
}

pub(crate) fn move_bullet(mut query: Query<(&mut Transform, &MoveDir), With<Bullet>>) {
    for (mut transform, dir) in query.iter_mut() {
        let delta = (dir.0 * 35.0).extend(0.);
        transform.translation += delta;
    }
}

// Very inaccurate. It make it more "realistic"!
const PLAYER_RADIUS: f32 = 24.0;
const BULLET_RADIUS: f32 = 0.25;

pub(crate) fn kill_players(
    mut commands: Commands,
    mut state: ResMut<State<GameState>>,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<Bullet>)>,
    bullet_query: Query<&Transform, With<Bullet>>,
) {
    for (player, player_transform) in player_query.iter() {
        for bullet_transform in bullet_query.iter() {
            let distance = Vec2::distance(
                player_transform.translation.xy(),
                bullet_transform.translation.xy(),
            );
            if distance < PLAYER_RADIUS + BULLET_RADIUS {
                commands.entity(player).despawn_recursive();
                let _ = state.set(GameState::Interlude);
            }
        }
    }
}

pub(crate) fn camera_follow(
    player_handle: Option<Res<LocalPlayerHandle>>,
    player_query: Query<(&Player, &Transform)>,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    let player_handle = match player_handle {
        Some(handle) => handle.0,
        None => return, // Session hasn't started yet
    };

    for (player, player_transform) in player_query.iter() {
        if player.as_idx() != player_handle.as_idx() {
            continue;
        }

        let pos = player_transform.translation;

        for mut transform in camera_query.iter_mut() {
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }
    }
}
