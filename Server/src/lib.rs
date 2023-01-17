use spacetimedb::{println, spacetimedb, Hash};

#[spacetimedb(table)]
pub struct PlayerComponent {
    #[unique]
    pub entity_id: u64,
    #[unique]
    pub owner_id: Hash,
    /// The input encodes all the moves/fire state of the player
    pub input: u8,
}

#[spacetimedb(reducer)]
pub fn create_new_player(identity: Hash, _timestamp: u64, entity_id: u64, input: u8) {
    // Make sure this player doesn't already exist
    if PlayerComponent::filter_by_entity_id(entity_id).is_some() {
        println!("A player with this entity_id already exists: {}", entity_id);
    } else {
        println!("Creating player with this ID: {}", entity_id);
        PlayerComponent::insert(PlayerComponent {
            entity_id,
            owner_id: identity,
            input,
        });
        println!("Player created: {}", entity_id);
    }
}

#[spacetimedb(reducer)]
pub fn move_player(identity: Hash, entity_id: u64, input: u8) {
    if entity_id > 2 {
        panic!("This is a 2 player game, so entity_id <= 2");
    }

    let player =
        PlayerComponent::filter_by_entity_id(entity_id).expect("This player doesn't exist.");

    // Make sure this identity owns this player
    if player.owner_id != identity {
        println!("This identity doesn't own this player! (allowed for now)");
    }

    PlayerComponent::update_by_entity_id(
        entity_id,
        PlayerComponent {
            entity_id,
            owner_id: identity,
            input,
        },
    );
}

// #[spacetimedb(disconnect)]
// pub fn identity_connected(identity: Hash, _timestamp: u64) {
//     println!("{}", identity);
// }
//
// #[spacetimedb(disconnect)]
// pub fn identity_disconnected(identity: Hash, _timestamp: u64) {
//     println!("{}", identity);
// }
