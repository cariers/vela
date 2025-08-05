use std::{
    collections::VecDeque,
    convert::Infallible,
    io,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use futures::{FutureExt, SinkExt, StreamExt, future::BoxFuture, ready};
use vela_core::protobuf;
use volans::{
    core::{PeerId, Url, upgrade::ReadyUpgrade},
    swarm::{
        BehaviorEvent, ConnectionDenied, ConnectionHandler, ConnectionHandlerEvent, ConnectionId,
        InboundStreamHandler, InboundUpgradeSend, NetworkBehavior, NetworkIncomingBehavior,
        StreamProtocol, Substream, SubstreamProtocol, THandlerAction, THandlerEvent,
    },
};

use crate::protocol;

pub struct Handler {
    joined: Option<String>,
    stream:
        Option<protobuf::IoStream<protocol::IncomingMessage, protocol::OutgoingMessage, Substream>>,
    pending: VecDeque<protocol::outgoing_message::Message>,
}

// struct ActiveStream {
//     table_id: Arc<String>,
//     stream: protocol::SocketStream<Substream>,
// }

impl Handler {
    fn cleanup(&mut self) {
        self.joined = None;
        self.stream = None;
        self.pending.clear();
    }
}

#[derive(Debug)]
pub enum Event {
    Message(protocol::incoming_message::Message),
    IoError(io::Error),
}

// impl ConnectionHandler for Handler {
//     type Action = Infallible;

//     type Event = Event;

//     fn handle_action(&mut self, _action: Self::Action) {
//         unreachable!("Connect handler does not support actions");
//     }

//     fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ConnectionHandlerEvent<Self::Event>> {
//         let this = &mut *self;

//         loop {
//             // 写入消息
//             if let Some(socket) = this.stream.as_mut() {
//                 if socket.poll_ready_unpin(cx).is_ready() {
//                     if let Some(message) = this.pending.pop_front() {
//                         if let Err(e) = socket.start_send_unpin(message) {
//                             self.cleanup();
//                             return Poll::Ready(ConnectionHandlerEvent::Notify(Event::IoError(e)));
//                         }
//                     }
//                 }
//                 // 刷新写入缓冲区
//                 match socket.poll_flush_unpin(cx) {
//                     Poll::Ready(Ok(())) => {}
//                     Poll::Ready(Err(e)) => {
//                         self.cleanup();
//                         return Poll::Ready(ConnectionHandlerEvent::Notify(Event::IoError(e)));
//                     }
//                     Poll::Pending => {}
//                 }
//                 // 读取消息
//                 match socket.poll_next_unpin(cx) {
//                     Poll::Ready(Some(Ok(message))) => {
//                         return Poll::Ready(ConnectionHandlerEvent::Notify(Event::Message(
//                             message,
//                         )));
//                     }
//                     Poll::Ready(Some(Err(e))) => {
//                         self.cleanup();
//                         return Poll::Ready(ConnectionHandlerEvent::Notify(Event::IoError(e)));
//                     }
//                     Poll::Ready(None) => {
//                         self.cleanup();
//                         return Poll::Ready(ConnectionHandlerEvent::CloseConnection);
//                     }
//                     Poll::Pending => {}
//                 }
//             }
//             return Poll::Pending;
//         }
//     }
// }

// impl InboundStreamHandler for Handler {
//     type InboundUpgrade = protocol::Upgrade;

//     type InboundUserData = ();

//     fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundUpgrade, Self::InboundUserData> {
//         SubstreamProtocol::new(protocol::Upgrade, ())
//     }

//     fn on_fully_negotiated(
//         &mut self,
//         _user_data: Self::InboundUserData,
//         stream: <Self::InboundUpgrade as InboundUpgradeSend>::Output,
//     ) {
//         self.stream = Some(protocol::SocketStream::new(stream));
//     }

//     fn on_upgrade_error(
//         &mut self,
//         _user_data: Self::InboundUserData,
//         _error: <Self::InboundUpgrade as InboundUpgradeSend>::Error,
//     ) {
//         self.cleanup();
//     }
// }
