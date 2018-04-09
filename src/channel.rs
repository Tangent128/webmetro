use std::sync::{
    Arc,
    Mutex
};

use futures::{
    Async,
    AsyncSink,
    Sink,
    Stream,
    sync::mpsc::{
        channel as mpsc_channel,
        Sender,
        Receiver
    }
};
use odds::vec::VecExt;

use chunk::Chunk;

/// A collection of listeners to a stream of WebM chunks.
/// Sending a chunk may fail due to a client being disconnected,
/// or simply failing to keep up with the stream buffer. In either
/// case, there's nothing practical the server can do to recover,
/// so the failing client is just dropped from the listener list.
pub struct Channel {
    header_chunk: Option<Chunk>,
    listeners: Vec<Sender<Chunk>>
}

impl Channel {
    pub fn new() -> Arc<Mutex<Channel>> {
        Arc::new(Mutex::new(Channel {
            header_chunk: None,
            listeners: Vec::new()
        }))
    }
}

pub struct Transmitter {
    channel: Arc<Mutex<Channel>>
}

impl Sink for Transmitter {
    type SinkItem = Chunk;
    type SinkError = (); // never errors, slow clients are simply dropped

    fn start_send(&mut self, chunk: Chunk) -> Result<AsyncSink<Chunk>, ()> {
        let mut channel = self.channel.lock().expect("Locking channel");

        if let Chunk::Headers { .. } = chunk {
            channel.header_chunk = Some(chunk.clone());
        }

        channel.listeners.retain_mut(|listener| listener.start_send(chunk.clone()).is_ok());

        Ok(AsyncSink::Ready)
    }
    fn poll_complete(&mut self) -> Result<Async<()>, ()> {
        let mut channel = self.channel.lock().expect("Locking channel");

        channel.listeners.retain_mut(|listener| listener.poll_complete().is_ok());

        Ok(Async::Ready(()))
    }
}

pub struct Listener {
    /// not used in operation, but its refcount keeps the channel alive when there's no Transmitter
    _channel: Arc<Mutex<Channel>>,
    receiver: Receiver<Chunk>
}

impl Listener {
    pub fn new(channel_arc: Arc<Mutex<Channel>>) -> Self {
        let (mut sender, receiver) = mpsc_channel(5);

        {
            let mut channel = channel_arc.lock().expect("Locking channel");

            if let Some(ref chunk) = channel.header_chunk {
                sender.start_send(chunk.clone()).expect("Queuing existing header chunk");
            }

            channel.listeners.push(sender);
        }

        Listener {
            _channel: channel_arc,
            receiver: receiver
        }
    }
}

impl Stream for Listener {
    type Item = Chunk;
    type Error = (); // no transmitter errors are exposed to the listeners

    fn poll(&mut self) -> Result<Async<Option<Chunk>>, ()> {
        self.receiver.poll()
    }
}
