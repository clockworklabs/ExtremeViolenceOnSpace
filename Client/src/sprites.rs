use bevy::prelude::*;

// A timer for animations
#[derive(Component, Deref, DerefMut)]
pub(crate) struct AnimationTimer(pub(crate) Timer);

// How the animation should continue after it reaches the last frame
pub(crate) enum AnimationStyle {
    Once,    // Play once and end at last frame
    Looping, // Loop from frame 1 to n, then from 1 to n, ad infinitum
}

pub(crate) fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&mut AnimationTimer, &mut TextureAtlasSprite)>,
) {
    for (mut timer, mut sprite) in &mut query {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            sprite.index = (sprite.index + 1) % 4;
        }
    }
}
