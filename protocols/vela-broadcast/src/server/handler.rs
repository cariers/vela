use std::{
    convert::Infallible,
    fmt, mem,
    task::{Context, Poll},
};

use async_broadcast::Receiver;
use futures::{SinkExt, StreamExt};
use volans::{
    codec::{Encoder, FramedWrite},
    core::upgrade::ReadyUpgrade,
    swarm::{
        ConnectionHandler, ConnectionHandlerEvent, InboundStreamHandler, InboundUpgradeSend,
        StreamProtocol, Substream, SubstreamProtocol,
    },
};

pub struct Handler<TCodec>
where
    TCodec: Encoder + Clone + Send + 'static,
    TCodec::Item<'static>: fmt::Debug + Clone + Send,
    TCodec::Error: fmt::Debug + Send,
{
    protocol: StreamProtocol,
    broadcast: Receiver<TCodec::Item<'static>>,
    active_stream: SendState<TCodec>,
    codec: TCodec,
}

impl<TCodec> Handler<TCodec>
where
    TCodec: Encoder + Clone + Send + 'static,
    TCodec::Item<'static>: fmt::Debug + Clone + Send,
    TCodec::Error: fmt::Debug + Send,
{
    pub(crate) fn new(
        protocol: StreamProtocol,
        broadcast: Receiver<TCodec::Item<'static>>,
        codec: TCodec,
    ) -> Self {
        Self {
            protocol,
            broadcast,
            active_stream: SendState::None,
            codec,
        }
    }
}

impl<TCodec> ConnectionHandler for Handler<TCodec>
where
    TCodec: Encoder + Clone + Send + 'static,
    TCodec::Item<'static>: fmt::Debug + Clone + Send,
    TCodec::Error: fmt::Debug + Send,
{
    type Action = Infallible;
    type Event = Infallible;

    fn handle_action(&mut self, _action: Self::Action) {
        unreachable!("This handler does not support actions");
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ConnectionHandlerEvent<Self::Event>> {
        loop {
            match mem::replace(&mut self.active_stream, SendState::None) {
                SendState::None => {}
                SendState::Receiving {
                    stream,
                    mut receiver,
                } => {
                    // 接收要广播的消息
                    match receiver.poll_next_unpin(cx) {
                        Poll::Ready(Some(message)) => {
                            self.active_stream = SendState::Sending {
                                stream: stream,
                                message,
                                receiver,
                            };
                            continue;
                        }
                        Poll::Ready(None) => {
                            self.active_stream = SendState::None;
                            tracing::debug!("Broadcast channel closed");
                        }
                        Poll::Pending => {
                            self.active_stream = SendState::Receiving { stream, receiver };
                        }
                    }
                }
                SendState::Sending {
                    mut stream,
                    receiver,
                    message,
                } => match stream.poll_ready_unpin(cx) {
                    Poll::Ready(Ok(())) => match stream.start_send_unpin(message.clone()) {
                        Ok(()) => {
                            tracing::debug!("Send broadcast message: {:?}", message);
                            // 重置接收状态
                            self.active_stream = SendState::Flushing { stream, receiver };
                            continue;
                        }
                        Err(e) => {
                            tracing::debug!("Error sending message: {:?}", e);
                            self.active_stream = SendState::None;
                        }
                    },
                    Poll::Ready(Err(e)) => {
                        tracing::debug!("Error sending message: {:?}", e);
                        self.active_stream = SendState::None;
                    }
                    Poll::Pending => {
                        self.active_stream = SendState::Sending {
                            stream,
                            receiver,
                            message,
                        };
                    }
                },
                SendState::Flushing {
                    mut stream,
                    receiver,
                } => match stream.poll_flush_unpin(cx) {
                    Poll::Ready(Ok(())) => {
                        tracing::debug!("Flushed broadcast message");
                        self.active_stream = SendState::Receiving { stream, receiver };
                        continue;
                    }
                    Poll::Ready(Err(e)) => {
                        tracing::debug!("Error flushing broadcast message: {:?}", e);
                    }
                    Poll::Pending => {
                        self.active_stream = SendState::Flushing { stream, receiver };
                    }
                },
            }
            return Poll::Pending;
        }
    }
}

impl<TCodec> InboundStreamHandler for Handler<TCodec>
where
    TCodec: Encoder + Clone + Send + 'static,
    TCodec::Item<'static>: fmt::Debug + Clone + Send,
    TCodec::Error: fmt::Debug + Send,
{
    type InboundUpgrade = ReadyUpgrade<StreamProtocol>;

    type InboundUserData = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundUpgrade, Self::InboundUserData> {
        SubstreamProtocol::new(ReadyUpgrade::new(self.protocol.clone()), ())
    }

    fn on_fully_negotiated(
        &mut self,
        _user_data: Self::InboundUserData,
        stream: <Self::InboundUpgrade as InboundUpgradeSend>::Output,
    ) {
        let receiver = self.broadcast.clone();
        let framed = FramedWrite::new(stream, self.codec.clone());
        self.active_stream = SendState::Receiving {
            stream: framed,
            receiver,
        };
        tracing::debug!("New inbound substream Broadcast");
    }

    fn on_upgrade_error(
        &mut self,
        _user_data: Self::InboundUserData,
        error: <Self::InboundUpgrade as InboundUpgradeSend>::Error,
    ) {
        tracing::debug!("Upgrade error: {:?}", error);
    }
}

enum SendState<TCodec>
where
    TCodec: Encoder + Clone + Send + 'static,
    TCodec::Item<'static>: fmt::Debug + Clone + Send,
{
    None,
    Receiving {
        stream: FramedWrite<Substream, TCodec>,
        receiver: Receiver<TCodec::Item<'static>>,
    },
    Sending {
        stream: FramedWrite<Substream, TCodec>,
        receiver: Receiver<TCodec::Item<'static>>,
        message: TCodec::Item<'static>,
    },
    Flushing {
        stream: FramedWrite<Substream, TCodec>,
        receiver: Receiver<TCodec::Item<'static>>,
    },
}
