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

impl Default for PlayerId {
    fn default() -> Self {
        Self::One
    }
}

impl PlayerId {
    pub fn as_idx(&self) -> usize {
        match self {
            PlayerId::One => 0,
            PlayerId::Two => 1,
        }
    }
}

#[derive(Debug, Component, PartialEq, Eq)]
pub(crate) struct Player {
    pub(crate) handle: PlayerId,
    // 4-directions + fire fits easily in a single byte
    pub(crate) input: u8,
}

impl Player {
    pub fn new(handle: PlayerId) -> Self {
        Self { handle, input: 0 }
    }

    pub fn as_idx(&self) -> usize {
        self.handle.as_idx()
    }
}

/// Add the player to the game SpaceTimeDb instance
pub(crate) fn create_new_player(db: &Arc<Client>, player: PlayerId, _client_id: &ConnectionHandle) {
    db.send_message(SpaceDbRequest::FunctionCall {
        name: "create_new_player".to_string(),
        args: vec![TypeValue::U32(player.as_idx() as u32), TypeValue::U8(0)],
    });
}

/// Updates the player state in the SpaceTimeDb instance
pub(crate) fn move_player(db: &Arc<Client>, player: PlayerId, input: u8) {
    db.send_message(SpaceDbRequest::FunctionCall {
        name: "move_player".to_string(),
        args: vec![TypeValue::U32(player.as_idx() as u32), TypeValue::U8(input)],
    });
}
