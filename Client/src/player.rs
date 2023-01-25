use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy_ggrs::RollbackIdProvider;
use bevy_ggrs::*;

use crate::components::*;
use crate::database::*;
use crate::input::{direction, fire};
use crate::net::GgrsConfig;
use crate::sprites::{AnimationTimer, ImageAssets, SpritesheetAnimator};
use crate::{GameState, MAP_SIZE};

fn spawn_player(
    commands: &mut Commands,
    asset: &Res<ImageAssets>,
    rip: &mut ResMut<RollbackIdProvider>,
    player: PlayerId,
) {
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
        .insert(Player { handle: player })
        .insert(player_animations)
        .insert(BulletReady(true))
        .insert(MoveDir(move_dir))
        .insert(Rollback::new(rip.next_id()));
}

pub(crate) fn spawn_players(
    mut commands: Commands,
    asset_server: Res<ImageAssets>,
    mut rip: ResMut<RollbackIdProvider>,
    player_query: Query<Entity, With<Player>>,
    bullet_query: Query<Entity, With<Bullet>>,
) {
    for player in player_query.iter() {
        commands.entity(player).despawn_recursive();
    }
    for bullet in bullet_query.iter() {
        commands.entity(bullet).despawn_recursive();
    }

    dbg!("spawn");
    spawn_player(&mut commands, &asset_server, &mut rip, PlayerId::One);
    spawn_player(&mut commands, &asset_server, &mut rip, PlayerId::Two);
}

pub(crate) fn move_players(
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut player_query: Query<(
        &mut SpritesheetAnimator,
        &mut TextureAtlasSprite,
        &mut Transform,
        &mut MoveDir,
        &Player,
    )>,
) {
    for (mut animator, mut sprite, mut transform, mut move_direction, player) in
        player_query.iter_mut()
    {
        let (input, _) = inputs[player.handle as usize];
        let (direction, animation) = direction(animator.animation, input);
        animator.set_state(animation, &mut sprite);

        if direction == Vec2::ZERO {
            continue;
        }

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

pub(crate) fn reload_bullet(
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut query: Query<(&mut BulletReady, &Player)>,
) {
    for (mut can_fire, player) in query.iter_mut() {
        let (input, _) = inputs[player.handle as usize];
        if !fire(input) {
            can_fire.0 = true;
        }
    }
}

pub(crate) fn fire_bullets(
    mut commands: Commands,
    inputs: Res<PlayerInputs<GgrsConfig>>,
    images: Res<ImageAssets>,
    mut player_query: Query<(&Transform, &Player, &mut BulletReady, &MoveDir)>,
    mut rip: ResMut<RollbackIdProvider>,
) {
    for (transform, player, mut bullet_ready, move_dir) in player_query.iter_mut() {
        let (input, _) = inputs[player.handle as usize];
        //dbg!(fire(input), bullet_ready.0);
        if fire(input) && bullet_ready.0 {
            let player_pos = transform.translation.xy();
            let pos = player_pos + move_dir.0 * PLAYER_RADIUS + BULLET_RADIUS;
            commands.spawn((
                Bullet,
                Rollback::new(rip.next_id()),
                *move_dir,
                SpriteBundle {
                    transform: Transform::from_translation(pos.extend(200.))
                        .with_rotation(Quat::from_rotation_arc_2d(Vec2::X, move_dir.0)),
                    texture: images.bullet.clone(),
                    sprite: Sprite {
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

const PLAYER_RADIUS: f32 = 0.5;
const BULLET_RADIUS: f32 = 0.025;

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
