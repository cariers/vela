use std::{
    collections::VecDeque,
    convert::Infallible,
    fmt, io, mem,
    task::{Context, Poll},
};

use futures::StreamExt;
use volans::{
    codec::{Decoder, FramedRead},
    core::upgrade::ReadyUpgrade,
    swarm::{
        ConnectionHandler, ConnectionHandlerEvent, OutboundStreamHandler, OutboundUpgradeSend,
        StreamProtocol, StreamUpgradeError, Substream, SubstreamProtocol,
    },
};

pub struct Handler<TCodec>
where
    TCodec: Decoder + Clone + Send + 'static,
    TCodec::Item: fmt::Debug + Send,
    TCodec::Error: fmt::Debug + Send + Into<io::Error>,
{
    protocol: StreamProtocol,
    active_stream: ReceiverState<TCodec>,
    pending_events: VecDeque<Event<TCodec::Item>>,
    codec: TCodec,
}

impl<TCodec> Handler<TCodec>
where
    TCodec: Decoder + Clone + Send + 'static,
    TCodec::Item: fmt::Debug + Send,
    TCodec::Error: fmt::Debug + Send + Into<io::Error>,
{
    pub(crate) fn new(protocol: StreamProtocol, codec: TCodec) -> Self {
        Self {
            protocol,
            active_stream: ReceiverState::None,
            pending_events: VecDeque::new(),
            codec,
        }
    }
}

impl<TCodec> ConnectionHandler for Handler<TCodec>
where
    TCodec: Decoder + Clone + Send + 'static,
    TCodec::Item: fmt::Debug + Send,
    TCodec::Error: fmt::Debug + Send + Into<io::Error>,
{
    type Action = Infallible;
    type Event = Event<TCodec::Item>;

    fn handle_action(&mut self, _action: Self::Action) {
        unreachable!("This handler does not support actions");
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ConnectionHandlerEvent<Self::Event>> {
        loop {
            // Check for pending events
            if let Some(event) = self.pending_events.pop_front() {
                return Poll::Ready(ConnectionHandlerEvent::Notify(event));
            }
            // 读取状态机
            match mem::replace(&mut self.active_stream, ReceiverState::None) {
                ReceiverState::None => {}
                ReceiverState::OpenStream => {
                    self.active_stream = ReceiverState::OpenStream;
                }
                ReceiverState::Receiving { mut stream } => match stream.poll_next_unpin(cx) {
                    Poll::Ready(Some(Ok(msg))) => {
                        self.active_stream = ReceiverState::Receiving { stream };
                        return Poll::Ready(ConnectionHandlerEvent::Notify(Event::Message(msg)));
                    }
                    Poll::Ready(Some(Err(err))) => {
                        return Poll::Ready(ConnectionHandlerEvent::Notify(Event::IoError(
                            err.into(),
                        )));
                    }
                    Poll::Ready(None) => {
                        return Poll::Ready(ConnectionHandlerEvent::Notify(Event::Closed));
                    }
                    Poll::Pending => {
                        self.active_stream = ReceiverState::Receiving { stream };
                    }
                },
            }
            return Poll::Pending;
        }
    }
}

impl<TCodec> OutboundStreamHandler for Handler<TCodec>
where
    TCodec: Decoder + Clone + Send + 'static,
    TCodec::Item: fmt::Debug + Send,
    TCodec::Error: fmt::Debug + Send + Into<io::Error>,
{
    type OutboundUpgrade = ReadyUpgrade<StreamProtocol>;
    type OutboundUserData = ();

    fn on_fully_negotiated(
        &mut self,
        _user_data: Self::OutboundUserData,
        substream: <Self::OutboundUpgrade as OutboundUpgradeSend>::Output,
    ) {
        self.active_stream = ReceiverState::Receiving {
            stream: FramedRead::new(substream, self.codec.clone()),
        };
    }

    fn on_upgrade_error(
        &mut self,
        _user_data: Self::OutboundUserData,
        error: StreamUpgradeError<<Self::OutboundUpgrade as OutboundUpgradeSend>::Error>,
    ) {
        self.active_stream = ReceiverState::None;
        match error {
            StreamUpgradeError::NegotiationFailed => {
                self.pending_events.push_back(Event::Unsupported);
            }
            StreamUpgradeError::Timeout => {
                self.pending_events.push_back(Event::IoError(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "stream upgrade timed out",
                )));
            }
            StreamUpgradeError::Io(e) => {
                self.pending_events.push_back(Event::IoError(e));
            }
            StreamUpgradeError::Apply(_) => unreachable!("stream upgrade apply error unreachable"),
        }
    }

    fn poll_outbound_request(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<SubstreamProtocol<Self::OutboundUpgrade, Self::OutboundUserData>> {
        if matches!(self.active_stream, ReceiverState::None) {
            let upgrade = ReadyUpgrade::new(self.protocol.clone());
            self.active_stream = ReceiverState::OpenStream;
            return Poll::Ready(SubstreamProtocol::new(upgrade, ()));
        }
        Poll::Pending
    }
}

enum ReceiverState<TCodec> {
    None,
    OpenStream,
    Receiving {
        stream: FramedRead<Substream, TCodec>,
    },
}

pub enum Event<T> {
    Message(T),
    Unsupported,
    IoError(io::Error),
    Closed,
}

impl<T> fmt::Debug for Event<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::Message(m) => write!(f, "Event::Message({:?})", m),
            Event::Unsupported => write!(f, "Event::Unsupported"),
            Event::IoError(err) => write!(f, "Event::IoError({:?})", err),
            Event::Closed => write!(f, "Event::Closed"),
        }
    }
}
