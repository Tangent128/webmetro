use std::pin::Pin;
use std::task::{
    Context,
    Poll
};
use std::sync::{
    Arc,
    Mutex
};

use futures::{
    channel::mpsc::{
        channel as mpsc_channel,
        Sender,
        Receiver
    },
    Sink,
    Stream,
    never::Never,
};
use odds::vec::VecExt;

use crate::chunk::Chunk;

/// A collection of listeners to a stream of WebM chunks.
/// Sending a chunk may fail due to a client being disconnected,
/// or simply failing to keep up with the stream buffer. In either
/// case, there's nothing practical the server can do to recover,
/// so the failing client is just dropped from the listener list.
pub struct Channel {
    pub name: String,
    header_chunk: Option<Chunk>,
    listeners: Vec<Sender<Chunk>>
}

pub type Handle = Arc<Mutex<Channel>>;

impl Channel {
    pub fn new(name: String) -> Handle {
        Arc::new(Mutex::new(Channel {
            name,
            header_chunk: None,
            listeners: Vec::new()
        }))
    }
}

pub struct Transmitter {
    channel: Handle
}

impl Transmitter {
    pub fn new(channel_arc: Handle) -> Self {
        Transmitter {
            channel: channel_arc
        }
    }
}

impl Sink<Chunk> for Transmitter {
    type Error = Never; // never errors, slow clients are simply dropped

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Never>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, chunk: Chunk) -> Result<(), Never> {
        let mut channel = self.channel.lock().expect("Locking channel");

        if let Chunk::Headers { .. } = chunk {
            channel.header_chunk = Some(chunk.clone());
        }

        channel.listeners.retain_mut(|listener| listener.start_send(chunk.clone()).is_ok());

        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Never>> {
        let mut channel = self.channel.lock().expect("Locking channel");
        let mut result = Poll::Ready(Ok(()));

        // just disconnect any erroring listeners
        channel.listeners.retain_mut(|listener| match Pin::new(listener).poll_flush(cx) {
            Poll::Pending => {result = Poll::Pending; true},
            Poll::Ready(Ok(())) => true,
            Poll::Ready(Err(_)) => false,
        });

        result
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Never>> {
        // don't actually disconnect listeners, since other sources may want to transmit to this channel;
        // just ensure we've sent everything we can out
        self.poll_flush(cx)
    }
}

pub struct Listener {
    /// not used in operation, but its refcount keeps the channel alive when there's no Transmitter
    _channel: Handle,
    receiver: Receiver<Chunk>
}

impl Listener {
    pub fn new(channel_arc: Handle) -> Self {
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

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Chunk>> {
        let receiver = &mut self.get_mut().receiver;
        Pin::new(receiver).poll_next(cx)
    }
}
