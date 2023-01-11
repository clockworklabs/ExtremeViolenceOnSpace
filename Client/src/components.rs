use bevy::prelude::*;

#[derive(Component, Reflect, Default)]
pub struct BulletReady(pub bool);

#[derive(Component, Reflect, Default)]
pub struct Bullet;

#[derive(Component, Resource, Reflect, Default, Clone, Copy)]
pub struct MoveDir(pub Vec2);
