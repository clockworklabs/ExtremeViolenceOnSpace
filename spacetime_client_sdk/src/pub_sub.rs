use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::broadcast;
use tokio::sync::broadcast::Receiver;

/// The pub-sub channel is created with a capacity of `MAX_PUB_SUB_CONNECTIONS`.
///
/// A message is stored in the channel until **all** subscribers
/// have seen it.
///
/// ## WARNING!
///
/// A slow subscriber could result in messages being held indefinitely.
///
/// Publishing will result in old messages being dropped if capacity fills up.
///
/// This prevents slow consumers from blocking the entire system.
const MAX_PUB_SUB_CONNECTIONS: usize = 1024;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Channel {
    pub identity: u32,
    pub address: String,
}

impl Channel {
    pub fn new(identity: u32, address: &str) -> Self {
        Self {
            identity,
            address: address.to_string(),
        }
    }
}

/// Manage the pub-sub connections
#[derive(Debug, Clone)]
pub struct PubSubDb {
    // Using std Mutex instead of tokio because here we are not doing async
    state: Arc<Mutex<State>>,
}

impl PubSubDb {
    /// Create a new, empty, [PubSubDb] instance.
    pub fn new() -> PubSubDb {
        let state = Arc::new(Mutex::new(State {
            pub_sub: HashMap::new(),
            clients: HashMap::new(),
        }));

        PubSubDb { state }
    }

    pub fn state_lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }

    pub fn len(&self) -> usize {
        self.state_lock().pub_sub.len()
    }

    /// Returns a `Receiver` for the requested channel.
    ///
    /// The returned `Receiver` is used to receive values broadcast by [Self::publish].
    pub fn subscribe(&self, channel: Channel) -> Receiver<Msg<String>> {
        use std::collections::hash_map::Entry;

        let mut state = self.state_lock();

        // If there is no entry for the requested channel, then create a new
        // broadcast channel and associate it with the key. If one already
        // exists, return an associated receiver.
        let rec = match state.pub_sub.entry(channel.clone()) {
            Entry::Occupied(e) => e.get().subscribe(),
            Entry::Vacant(e) => {
                let (tx, rx) = broadcast::channel(MAX_PUB_SUB_CONNECTIONS);
                e.insert(tx);
                rx
            }
        };

        state.clients.insert(channel.clone(), rec);
        state.clients[&channel].resubscribe()
        // state.clients[&channel] = rec;
        // &state.clients[&channel]

        // state
        //     .clients
        //     .entry(channel)
        //     .and_modify(|counter| *counter = rec)
        //     .or_insert(rec);
        // &state.clients[&channel]
        //
        // match state.clients.entry(channel) {
        //     Entry::Occupied(e) => e.get(),
        //     Entry::Vacant(e) => {
        //         e.insert(rec);
        //         &rec
        //     }
        // }
    }

    /// Publish a message to ALL channels.
    ///
    /// Returns the number of subscribers already listening on it.
    pub fn publish_all(&self, msg: Msg<String>) -> usize {
        let state = self.state_lock();
        let mut total = 0;
        for ch in state.clients.keys() {
            total += self.publish(ch.clone(), msg.clone())
        }

        total
    }

    /// Publish a message to the channel.
    ///
    /// Returns the number of subscribers already listening on it.
    pub fn publish(&self, channel: Channel, msg: Msg<String>) -> usize {
        let state = self.state_lock();

        // The number of subscribers is returned on successful send.
        // An error or a new channel indicates there are no
        // receivers, in which case, `0` should be returned.
        state
            .pub_sub
            .get(&channel)
            .map(|tx| tx.send(msg).unwrap_or(0))
            .unwrap_or(0)
    }

    pub fn listen(&self, channel: Channel) -> Receiver<Msg<String>> {
        let state = self.state_lock();

        state.clients[&channel].resubscribe()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Msg<T> {
    Ping,
    Pong,
    Op(T),
}

impl<T> Msg<T> {
    pub(crate) fn op(&self) -> Option<&T> {
        match self {
            Msg::Op(x) => Some(x),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct State {
    pub_sub: HashMap<Channel, broadcast::Sender<Msg<String>>>,
    pub clients: HashMap<Channel, broadcast::Receiver<Msg<String>>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use spacetimedb::spacetimedb_lib::error::ResultTest;
    use tokio::sync::broadcast::error::TryRecvError;

    #[tokio::test]
    async fn test_pub_sub() -> ResultTest<()> {
        let server = PubSubDb::new();

        let client = server.clone();
        let ch1 = Channel::new(0);
        let ch2 = Channel::new(1);

        let mut ret = client.subscribe(ch1);

        server.publish(ch1, Msg::Ping);
        server.publish(ch1, Msg::Ping);
        server.publish(ch2, Msg::Ping);

        assert_eq!(ret.try_recv(), Ok(Msg::Ping));
        assert_eq!(ret.try_recv(), Ok(Msg::Ping));
        assert_eq!(ret.try_recv(), Err(TryRecvError::Empty));

        //Joining later not see past messages!
        let mut ret = client.subscribe(ch2);
        assert_eq!(ret.try_recv(), Err(TryRecvError::Empty));

        Ok(())
    }
}
