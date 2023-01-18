/// Interface to the SpaceTimeDb database engine.
///
use bevy::prelude::*;
use spacetime_client_sdk::messages::SpaceDbRequest;
use spacetime_client_sdk::spacetimedb::TypeValue;
use spacetime_client_sdk::web_socket::{Client, ConnectionHandle};
use std::sync::Arc;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PlayerId {
    One,
    Two,
}

impl PlayerId {
    pub fn as_idx(&self) -> usize {
        match self {
            PlayerId::One => 0,
            PlayerId::Two => 1,
        }
    }
}

#[derive(Component, PartialEq, Eq)]
pub(crate) struct Player {
    pub(crate) handle: PlayerId,
}

impl Player {
    pub fn new(handle: PlayerId) -> Self {
        Self { handle }
    }

    pub fn as_idx(&self) -> usize {
        self.handle.as_idx()
    }
}

/// Add the player to the game SpaceTimeDb instance
pub(crate) fn create_new_player(db: &Arc<Client>, player: PlayerId, client_id: &ConnectionHandle) {
    db.send_message(SpaceDbRequest::FunctionCall {
        name: "create_new_player".to_string(),
        args: vec![TypeValue::U32(player.as_idx() as u32), TypeValue::U8(0)],
    });
}

/// Updates the player state in the SpaceTimeDb instance
pub(crate) fn move_player() {}
