use crate::database::{move_player, Player, PlayerId};
use crate::sprites::{Animation, Facing, SpritesheetAnimator};
use bevy::prelude::*;

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;
const INPUT_FIRE: u8 = 1 << 4;

pub fn input(keys: &Res<Input<KeyCode>>) -> u8 {
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
