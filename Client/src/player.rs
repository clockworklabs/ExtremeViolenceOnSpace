use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy_ggrs::RollbackIdProvider;
use bevy_ggrs::*;

use crate::components::*;
use crate::database::*;
use crate::input::{direction, fire};
use crate::sprites::{AnimationTimer, ImageAssets, SpritesheetAnimator};
use crate::{GgrsConfig, MAP_SIZE};

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
) {
    spawn_player(&mut commands, &asset_server, &mut rip, PlayerId::One);
    spawn_player(&mut commands, &asset_server, &mut rip, PlayerId::Two);
}

pub(crate) fn move_players(
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut player_query: Query<(&mut Transform, &mut MoveDir, &Player)>,
) {
    dbg!("move");
    for (mut transform, mut move_direction, player) in player_query.iter_mut() {
        dbg!("movedir");
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

pub(crate) fn move_players2(
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut player_query: Query<(&mut Transform, &mut MoveDir, &Player)>,
) {
    dbg!("move");
    for (mut transform, mut move_direction, player) in player_query.iter_mut() {
        dbg!("movedir");
        let (input, _) = inputs[player.as_idx()];
        let direction = direction(input);

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

fn fire_bullets(
    mut commands: Commands,
    inputs: Res<PlayerInputs<GgrsConfig>>,
    images: Res<ImageAssets>,
    player_query: Query<(&Transform, &Player)>,
) {
    for (transform, player) in player_query.iter() {
        // TODO: Check if player pressed fire button
        // Spawn bullet
    }
}
//
// fn reload_bullet(
//     inputs: Res<PlayerInputs<GgrsConfig>>,
//     mut query: Query<(&mut BulletReady, &Player)>,
// ) {
//     for (mut can_fire, player) in query.iter_mut() {
//         let (input, _) = inputs[player.handle];
//         if !fire(input) {
//             can_fire.0 = true;
//         }
//     }
// }

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalPlayerHandle(pub(crate) PlayerId);

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
