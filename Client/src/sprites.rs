use crate::database::PlayerId;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

//Must match  columns bellow
const SPRITE_FRAMES: usize = 5;
const DEFAULT_ANIMATION_FPS: f32 = 5.0;

#[derive(AssetCollection, Resource)]
pub(crate) struct ImageAssets {
    #[asset(path = "images/Bullet_cowboy.png")]
    bullet: Handle<Image>,
    #[asset(path = "images/Alien.png")]
    #[asset(texture_atlas(tile_size_x = 1127., tile_size_y = 1920., columns = 5, rows = 1))]
    pub(crate) alien: Handle<TextureAtlas>,
    #[asset(path = "images/CowBoy.png")]
    #[asset(texture_atlas(tile_size_x = 1127., tile_size_y = 1920., columns = 5, rows = 1))]
    pub(crate) cowboy: Handle<TextureAtlas>,
}

// A timer for animations
#[derive(Component, Deref, DerefMut)]
pub(crate) struct AnimationTimer(pub(crate) Timer);

#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum Facing {
    Left,
    Right,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum Animation {
    Idle(Facing),
    Run(Facing),
    Fire(Facing),
    Dead(Facing),
}

impl Animation {
    /// The animation frame indices start from 1, not 0.
    /// This is a choice: since we’re encoding “flip-x” as negative,
    /// we can’t use 0, or we couldn’t flip that frame.
    ///
    /// So, the frame indices here are +1 the TextureAtlas indices.
    fn frames(&self) -> &[i8] {
        match self {
            Animation::Idle(dir) => match dir {
                Facing::Left => &[-2],
                Facing::Right => &[2],
            },
            Animation::Run(dir) => match dir {
                Facing::Left => &[-1, -2, -3],
                Facing::Right => &[1, 2, 3],
            },
            Animation::Fire(dir) => match dir {
                Facing::Left => &[-1, -4],
                Facing::Right => &[1, 4],
            },
            Animation::Dead(dir) => match dir {
                Facing::Left => &[-5],
                Facing::Right => &[5],
            },
        }
    }

    pub fn facing(&self) -> Facing {
        match self {
            Animation::Idle(x) => *x,
            Animation::Run(x) => *x,
            Animation::Fire(x) => *x,
            Animation::Dead(x) => *x,
        }
    }

    pub fn change(&self, to: Facing) -> Self {
        match self {
            Animation::Idle(_) => Animation::Idle(to),
            Animation::Run(_) => Animation::Run(to),
            Animation::Fire(_) => Animation::Fire(to),
            Animation::Dead(_) => Animation::Dead(to),
        }
    }
}

#[derive(Component)]
pub(crate) struct SpritesheetAnimator {
    timer: AnimationTimer,
    pub(crate) animation: Animation,
    pub(crate) cur_frame_idx: usize,
    pub(crate) player: PlayerId,
}

impl SpritesheetAnimator {
    pub(crate) fn new(player: PlayerId) -> Self {
        let dir = match player {
            PlayerId::One => Facing::Left,
            PlayerId::Two => Facing::Right,
        };

        Self {
            timer: AnimationTimer(Timer::from_seconds(1.0 / 30.0, TimerMode::Repeating)),
            animation: Animation::Idle(dir),
            cur_frame_idx: 0,
            player,
        }
    }

    pub(crate) fn set_state(&mut self, animation: Animation, sprite: &mut TextureAtlasSprite) {
        // Stop looping the firing!
        self.animation = animation;
        // if matches!(animation, Animation::Fire(_)) {
        //     dbg!(self.cur_frame_idx, animation.frames().len());
        //     if self.cur_frame_idx == animation.frames().len() - 1 {
        //         self.animation = Animation::Still(animation.facing())
        //     }
        // }
        self.cur_frame_idx = 0;
        // Set the sprite frame and x-flip value
        if let Some(texture_idx) = self.animation.frames().get(0) {
            sprite.index = ((*texture_idx).abs() - 1) as usize;
            sprite.flip_x = (*texture_idx) < 0; // flip texture if negative
        }
    }
}

pub(crate) fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&mut SpritesheetAnimator, &mut TextureAtlasSprite)>,
) {
    for (mut animator, mut sprite) in &mut query {
        let timer = &mut animator.timer;
        timer.tick(time.delta());
        if timer.just_finished() {
            let frames = animator.animation.frames();

            // Get reference to current animation and advance to next frame
            let next_frame_idx: usize;
            // Advance to the index of the next frame
            let num_frames = frames.len();
            if (animator.cur_frame_idx + 1) >= num_frames {
                next_frame_idx = 0;
            } else {
                next_frame_idx = animator.cur_frame_idx + 1;
            }

            let texture_idx = frames[next_frame_idx];
            sprite.index = (((texture_idx).abs() - 1) as usize) % SPRITE_FRAMES;
            sprite.flip_x = (texture_idx) < 0; // flip texture if negative

            animator.cur_frame_idx = next_frame_idx;
        }
    }
}
