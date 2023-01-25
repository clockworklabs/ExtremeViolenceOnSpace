use crate::bevy_ws::WsClient;
use crate::database::{move_player, Player, PlayerId};
use crate::sprites::{Animation, Facing, SpritesheetAnimator};
use bevy::prelude::*;

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;
const INPUT_FIRE: u8 = 1 << 4;

pub fn input(_: In<ggrs::PlayerHandle>, keys: Res<Input<KeyCode>>) -> u8 {
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

pub(crate) fn direction(animation: Animation, input: u8) -> (Vec2, Animation) {
    let mut direction = Vec2::ZERO;
    let mut animation = Animation::Run(animation.facing());

    if input & INPUT_UP != 0 {
        direction.y += 1.;
    }
    if input & INPUT_DOWN != 0 {
        direction.y -= 1.;
    }
    if input & INPUT_RIGHT != 0 {
        direction.x += 1.;
        animation = animation.change(Facing::Right);
    }
    if input & INPUT_LEFT != 0 {
        direction.x -= 1.;
        animation = animation.change(Facing::Left);
    }
    (direction.normalize_or_zero(), animation)
}

pub fn fire(input: u8) -> bool {
    input & INPUT_FIRE != 0
}

pub(crate) fn input2(
    keys: Res<Input<KeyCode>>,
    mut socket: ResMut<WsClient>,

    mut player_query: Query<(
        &mut SpritesheetAnimator,
        &mut TextureAtlasSprite,
        &mut Transform,
        &Player,
    )>,
) {
    for (mut animator, mut sprite, mut transform, player) in player_query.iter_mut() {
        if player.handle != PlayerId::One {
            continue;
        }

        let (left_pressed, right_pressed) =
            (keys.pressed(KeyCode::Left), keys.pressed(KeyCode::Right));

        let mut input = 0u8;
        let mut direction = Vec2::ZERO;
        let mut animation = animator.animation;
        animation = Animation::Run(animation.facing());
        if keys.any_pressed([KeyCode::Up, KeyCode::W]) {
            input |= INPUT_UP;
            direction.y += 1.;
            animation = Animation::Run(animation.facing());
        }
        if keys.any_pressed([KeyCode::Down, KeyCode::S]) {
            input |= INPUT_DOWN;
            direction.y -= 1.;
            animation = Animation::Run(animation.facing());
        }
        if keys.any_pressed([KeyCode::Left, KeyCode::A]) {
            input |= INPUT_LEFT;
            direction.x -= 1.;
            animation = animation.change(Facing::Left);
        }
        if keys.any_pressed([KeyCode::Right, KeyCode::D]) {
            input |= INPUT_RIGHT;
            direction.x += 1.;
            animation = animation.change(Facing::Right);
        }
        if keys.any_pressed([KeyCode::Space, KeyCode::Return]) {
            input |= INPUT_FIRE;
            animation = Animation::Fire(animation.facing());
        }

        animator.set_state(animation, &mut sprite);

        if direction == Vec2::ZERO {
            return;
        }

        move_player(&socket.client, animator.player, input);
        //dbg!(input, direction);
        let move_speed = 20.13;
        let move_delta = (direction * move_speed).extend(0.);

        transform.translation += move_delta;
    }
}
