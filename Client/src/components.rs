use crate::database::PlayerId;
use bevy::prelude::*;

#[derive(Component, Reflect, Default)]
pub struct BulletReady(pub bool);

#[derive(Component, Reflect, Default)]
pub struct Bullet;

#[derive(Component, Resource, Reflect, Default, Clone, Copy)]
pub struct MoveDir(pub Vec2);

/// Used to help identify our main camera
#[derive(Component)]
pub struct MainCamera;

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalPlayerHandle(pub(crate) PlayerId);

#[derive(Resource, Default)]
pub(crate) struct InterludeTimer(pub(crate) usize);
