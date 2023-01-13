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
// use tokio::sync::mpsc::Sender;
//
// use futures_util::{SinkExt, StreamExt};
// use std::net::SocketAddr;
// use tokio::net::{TcpListener, TcpStream};
// use tokio_tungstenite::tungstenite::{Message, Result};
// use tokio_tungstenite::{accept_async, tungstenite::Error};

use async_compat::Compat;

use spacetime_client_sdk::pub_sub::PubSubDb;
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
    let room_url = "ws://127.0.0.1:3000";

    //Create the client to game channel, note the sender will be cloned by each connected client
    let (client_to_game_sender, client_to_game_receiver) = unbounded::<String>();

    let pub_sub = PubSubDb::new();

    //Spawn the tokio runtime setup using a Compat with the clients and client to game channel
    IoTaskPool::get()
        .spawn(Compat::new(tokio_setup(
            room_url,
            pub_sub.clone(),
            client_to_game_sender,
        )))
        .detach();

    //Insert the clients and client to game channel into the Bevy resources
    let client = ClientMSG {
        pub_sub,
        client_to_game_receiver,
    };
    commands.insert_resource(client);
}

pub(crate) fn message_system(client: Res<ClientMSG>) {
    //Broadcast a message to each connected client on each Bevy System iteration.
    // for (_id, client) in client.clients.lock().unwrap().iter() {
    //     println!("{:?}", client);
    //     client
    //         .sender
    //         .try_send("Broadcast message from Bevy System".to_string())
    //         .expect("Could not send message");
    // }

    //Attempts to receive a message from the channel without blocking.
    let state = client.pub_sub.state_lock();
    let chan = state.clients.keys().next().unwrap();

    if let Ok(msg) = client.pub_sub.listen(chan.clone()).try_recv() {
        println!("{:?}", msg);
    }
}

// async fn tokio_setup(clients: Clients, client_to_game_sender: CBSender<String>) {
//     let addr = "127.0.0.1:9002";
//     let listener = TcpListener::bind(&addr).await.expect("Can't listen");
//     println!("Listening on: {}", addr);
//
//     while let Ok((stream, _)) = listener.accept().await {
//         let peer = stream
//             .peer_addr()
//             .expect("connected streams should have a peer address");
//         println!("Peer address: {}", peer);
//
//         //Spawn a connection handler per client
//         tokio::spawn(accept_connection(
//             peer,
//             stream,
//             clients.clone(),
//             client_to_game_sender.clone(),
//         ));
//     }
//
//     println!("Finished");
// }
//
// async fn accept_connection(
//     peer: SocketAddr,
//     stream: TcpStream,
//     clients: Clients,
//     client_to_game_sender: CBSender<String>,
// ) {
//     if let Err(e) = handle_connection(peer, stream, clients, client_to_game_sender).await {
//         match e {
//             Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
//             err => println!("Error processing connection: {}", err),
//         }
//     }
// }
//
// async fn handle_connection(
//     peer: SocketAddr,
//     stream: TcpStream,
//     clients: Clients,
//     client_to_game_sender: CBSender<String>,
// ) -> Result<()> {
//     println!("New WebSocket connection: {}", peer);
//     let ws_stream = accept_async(stream).await.expect("Failed to accept");
//
//     let (mut ws_sender, mut ws_receiver) = ws_stream.split();
//
//     //Create a tokio sync channel to for messages from the game to each client
//     let (game_to_client_sender, mut game_to_client_receiver) = tokio::sync::mpsc::channel(100);
//
//     //Get the number of clients for a client id
//     let num_clients = clients.lock().unwrap().keys().len() as i32;
//
//     //Store the incremented client id and the game to client sender in the clients hashmap
//     clients.lock().unwrap().insert(
//         num_clients + 1,
//         Client {
//             id: num_clients + 1,
//             sender: game_to_client_sender,
//         },
//     );
//
//     //This loop uses the tokio select! macro to receive messages from either the websocket receiver
//     //or the game to client receiver
//     loop {
//         tokio::select! {
//             //Receive messages from the websocket
//             msg = ws_receiver.next() => {
//                 match msg {
//                     Some(msg) => {
//                         let msg = msg?;
//                         if msg.is_text() ||msg.is_binary() {
//                             client_to_game_sender.send(msg.to_string()).expect("Could not send message");
//                         } else if msg.is_close() {
//                             break;
//                         }
//                     }
//                     None => break,
//                 }
//             }
//             //Receive messages from the game
//             game_msg = game_to_client_receiver.recv() => {
//                 let game_msg = game_msg.unwrap();
//                 ws_sender.send(Message::Text(game_msg)).await?;
//             }
//
//         }
//     }
//
//     Ok(())
// }
//
// #[derive(Resource)]
// pub(crate) struct ClientMSG {
//     pub(crate) clients: Clients,
//     pub(crate) client_to_game_receiver: CBReceiver<String>,
// }
//
// fn main() {
//     App::new()
//         .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
//             TIMESTEP_5_PER_SECOND,
//         )))
//         .add_plugin(CorePlugin::default())
//         .add_plugin(ScheduleRunnerPlugin::default())
//         .add_startup_system(setup)
//         .add_system(message_system)
//         .run();
// }
