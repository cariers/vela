use std::{
    collections::VecDeque,
    task::{Context, Poll},
    time::Duration,
};

use futures::channel::{mpsc, oneshot};
use vela_core::{common, def_id, ids::SessionId};
use volans::{
    core::{Multiaddr, PeerId},
    swarm::{
        BehaviorEvent, ConnectionDenied, ConnectionId, NetworkBehavior, NetworkIncomingBehavior,
        StreamProtocol, THandlerAction, THandlerEvent,
    },
};

use crate::proto::{ClientMessage, ServerMessage};

mod handler;

pub mod proto;

const PROTOCOL_NAME: StreamProtocol = StreamProtocol::new("/v1/table");

const MAX_MESSAGE_SIZE: usize = 8 * 1024; // 8KB
const MAX_ACTIVE_DURATION: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

def_id!(TableId, "tb_");

#[derive(Debug)]
pub struct SessionAccepted {
    pub session_id: SessionId,
    pub table_id: TableId,
    pub event_sender: mpsc::Sender<ClientMessage>,
    pub event_receiver: mpsc::Receiver<ServerMessage>,
    pub table: proto::Table,
}

pub struct Behavior {
    pending_events: VecDeque<Event>,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            pending_events: VecDeque::new(),
        }
    }
}

impl NetworkBehavior for Behavior {
    type Event = Event;
    type ConnectionHandler = handler::Handler;

    fn on_connection_handler_event(
        &mut self,
        connection_id: ConnectionId,
        peer_id: PeerId,
        event: THandlerEvent<Self>,
    ) {
        let event = match event {
            handler::Event::Authenticate { request, responder } => Event::Authenticate {
                connection_id,
                peer_id,
                request,
                responder,
            },
            handler::Event::JoinAccepted {
                session_id,
                table_id,
            } => Event::JoinAccepted {
                connection_id,
                peer_id,
                session_id,
                table_id,
            },
        };
        self.pending_events.push_back(event);
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<BehaviorEvent<Self::Event, THandlerAction<Self>>> {
        if let Some(event) = self.pending_events.pop_front() {
            return Poll::Ready(BehaviorEvent::Behavior(event));
        }
        Poll::Pending
    }
}

impl NetworkIncomingBehavior for Behavior {
    fn handle_established_connection(
        &mut self,
        _id: ConnectionId,
        _peer_id: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        Ok(handler::Handler::new())
    }
}

#[derive(Debug)]
pub enum Event {
    Authenticate {
        connection_id: ConnectionId,
        peer_id: PeerId,
        request: proto::JoinTableReq,
        responder: oneshot::Sender<Result<SessionAccepted, common::Code>>,
    },
    JoinAccepted {
        connection_id: ConnectionId,
        peer_id: PeerId,
        session_id: SessionId,
        table_id: TableId,
    },
}
