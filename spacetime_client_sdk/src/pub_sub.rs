use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::broadcast;

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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(crate) struct Channel {
    table_id: u32,
    col_id: Option<u32>,
}

impl Channel {
    pub fn new(table_id: u32, col_id: Option<u32>) -> Self {
        Self { table_id, col_id }
    }
}

/// Manage the pub-sub connections
#[derive(Debug, Clone)]
pub(crate) struct PubSubDb {
    // Using std Mutex instead of tokio because here we are not doing async
    state: Arc<Mutex<State>>,
}

impl PubSubDb {
    /// Create a new, empty, [PubSubDb] instance.
    pub(crate) fn new() -> PubSubDb {
        let state = Arc::new(Mutex::new(State {
            pub_sub: HashMap::new(),
        }));

        PubSubDb { state }
    }

    fn state_lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }

    /// Returns a `Receiver` for the requested channel.
    ///
    /// The returned `Receiver` is used to receive values broadcast by [Self::publish].
    pub(crate) fn subscribe(&self, channel: Channel) -> broadcast::Receiver<Msg<String>> {
        use std::collections::hash_map::Entry;

        let mut state = self.state_lock();

        // If there is no entry for the requested channel, then create a new
        // broadcast channel and associate it with the key. If one already
        // exists, return an associated receiver.
        match state.pub_sub.entry(channel) {
            Entry::Occupied(e) => e.get().subscribe(),
            Entry::Vacant(e) => {
                let (tx, rx) = broadcast::channel(MAX_PUB_SUB_CONNECTIONS);
                e.insert(tx);
                rx
            }
        }
    }

    /// Publish a message to the channel.
    ///
    /// Returns the number of subscribers already listening on it.
    pub(crate) fn publish(&self, channel: Channel, msg: Msg<String>) -> usize {
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
struct State {
    pub_sub: HashMap<Channel, broadcast::Sender<Msg<String>>>,
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
        let ch1 = Channel::new(0, None);
        let ch2 = Channel::new(1, None);

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
