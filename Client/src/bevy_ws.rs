// Configure clippy for Bevy usage
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::enum_glob_use)]

use bevy::{
    app::ScheduleRunnerSettings, core::CorePlugin, prelude::*, tasks::IoTaskPool, utils::Duration,
};
use crossbeam_channel::{unbounded, Receiver as CBReceiver, Sender as CBSender};
use std::os::macos::raw::stat;
// use tokio::sync::mpsc::Sender;
//
// use futures_util::{SinkExt, StreamExt};
// use std::net::SocketAddr;
// use tokio::net::{TcpListener, TcpStream};
// use tokio_tungstenite::tungstenite::{Message, Result};
// use tokio_tungstenite::{accept_async, tungstenite::Error};

use async_compat::Compat;

use spacetime_client_sdk::pub_sub::{Msg, PubSubDb};
use spacetime_client_sdk::ws::tokio_setup;
use std::str::FromStr;

#[derive(Resource, Clone)]
pub(crate) struct ClientMSG {
    pub(crate) pub_sub: PubSubDb,
    pub(crate) client_to_game_receiver: CBReceiver<String>,
}

pub(crate) fn setup_net(mut commands: Commands) {
    println!("Bevy Setup System");
    let room_url = "ws://127.0.0.1:3000/database/subscribe?name_or_address=extremeviolenceonspace";

    //Create the client to game channel, note the sender will be cloned by each connected client
    let (client_to_game_sender, client_to_game_receiver) = unbounded::<String>();

    let pub_sub = PubSubDb::new();

    //Spawn the tokio runtime setup using a Compat with the clients and client to game channel
    IoTaskPool::get()
        .spawn(Compat::new(tokio_setup(room_url, pub_sub.clone())))
        .detach();
    //Insert the clients and client to game channel into the Bevy resources
    let client = ClientMSG {
        pub_sub,
        client_to_game_receiver,
    };
    commands.insert_resource(client);
}

pub(crate) fn message_system(client: Res<ClientMSG>) {
    let state = client.pub_sub.state_lock();
    //Broadcast a message to each connected client on each Bevy System iteration.
    for (_id, client) in state.clients.iter() {
        println!("{:?}", client);
        client
            .try_send(Msg::Op("Broadcast message from Bevy System".to_string()))
            .expect("Could not send message");
    }

    //Attempts to receive a message from the channel without blocking.
    //let chan = state.clients.keys().next().unwrap();

    if let Ok(msg) = state.client_to_game_receiver.try_recv() {
        println!("{:?}", msg);
    }
}
