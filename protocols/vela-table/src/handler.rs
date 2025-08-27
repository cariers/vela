use std::{
    convert::Infallible,
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures::{
    SinkExt, StreamExt,
    channel::{mpsc, oneshot},
};
use futures_bounded::{Delay, FuturesMap, FuturesSet};
use vela_core::{common, ids::SessionId};
use volans::{
    codec::{CombinedCodec, Framed, ProtobufUviCodec},
    core::upgrade::ReadyUpgrade,
    swarm::{
        ConnectionHandler, ConnectionHandlerEvent, InboundStreamHandler, InboundUpgradeSend,
        StreamProtocol, Substream, SubstreamProtocol,
    },
};

use crate::{
    MAX_ACTIVE_DURATION, MAX_MESSAGE_SIZE, PROTOCOL_NAME, SessionAccepted, TableId, proto,
};

pub struct Handler {
    pending_stream: FuturesSet<Result<Option<JoinedSession>, io::Error>>,
    authenticate_receiver: mpsc::Receiver<(
        proto::JoinTableReq,
        oneshot::Sender<Result<SessionAccepted, common::Code>>,
    )>,
    authenticate_sender: mpsc::Sender<(
        proto::JoinTableReq,
        oneshot::Sender<Result<SessionAccepted, common::Code>>,
    )>,
    joined_stream: FuturesMap<(TableId, SessionId), Result<(), io::Error>>,
}

impl Handler {
    pub(crate) fn new() -> Self {
        let (authenticate_sender, authenticate_receiver) = mpsc::channel(10);
        Self {
            pending_stream: FuturesSet::new(|| Delay::futures_timer(Duration::from_millis(3)), 5),
            authenticate_receiver,
            authenticate_sender,
            joined_stream: FuturesMap::new(|| Delay::futures_timer(MAX_ACTIVE_DURATION), 10),
        }
    }
}

impl ConnectionHandler for Handler {
    type Action = Infallible;
    type Event = Event;

    fn handle_action(&mut self, _action: Self::Action) {
        unreachable!("Unreachable Table Handler Action")
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ConnectionHandlerEvent<Self::Event>> {
        loop {
            match self.authenticate_receiver.poll_next_unpin(cx) {
                Poll::Ready(Some((request, responder))) => {
                    return Poll::Ready(ConnectionHandlerEvent::Notify(Event::Authenticate {
                        request,
                        responder,
                    }));
                }
                Poll::Pending | Poll::Ready(None) => {}
            }

            match self.pending_stream.poll_unpin(cx) {
                Poll::Ready(Ok(Ok(Some(session)))) => {
                    let session_id = session.session_id.clone();
                    let table_id = session.table_id.clone();
                    let res = self
                        .joined_stream
                        .try_push((table_id.clone(), session_id.clone()), session);
                    if res.is_err() {
                        tracing::warn!(
                            "Table Handler joined stream is full, dropping: {}-{}",
                            table_id,
                            session_id
                        );
                        continue;
                    }
                    let event = Event::JoinAccepted {
                        session_id: session_id,
                        table_id: table_id,
                    };
                    return Poll::Ready(ConnectionHandlerEvent::Notify(event));
                }
                Poll::Ready(Ok(Ok(None))) => {
                    tracing::info!("Table Handler joined session rejected");
                    continue;
                }
                Poll::Ready(Ok(Err(err))) => {
                    tracing::warn!("Table Handler pending stream error: {}", err);
                    continue;
                }
                Poll::Ready(Err(_)) => {
                    tracing::warn!("Table Handler pending stream error");
                    continue;
                }
                Poll::Pending => {}
            }
            match self.joined_stream.poll_unpin(cx) {
                Poll::Ready(((table_id, session_id), Ok(res))) => {
                    tracing::info!(
                        "Table Handler joined stream ended: {}-{}, Res: {:?}",
                        table_id,
                        session_id,
                        res
                    );
                    continue;
                }
                Poll::Ready(((table_id, session_id), Err(_))) => {
                    tracing::warn!(
                        "Table Handler joined stream keep long time: {}-{}",
                        table_id,
                        session_id
                    );
                    continue;
                }
                Poll::Pending => {}
            }
            return Poll::Pending;
        }
    }
}

impl InboundStreamHandler for Handler {
    type InboundUpgrade = ReadyUpgrade<StreamProtocol>;
    type InboundUserData = ();
    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundUpgrade, Self::InboundUserData> {
        return SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL_NAME), ());
    }

    fn on_fully_negotiated(
        &mut self,
        _user_data: Self::InboundUserData,
        io: <Self::InboundUpgrade as InboundUpgradeSend>::Output,
    ) {
        let res = self.pending_stream.try_push(make_join_table_request(
            io,
            self.authenticate_sender.clone(),
        ));
        if res.is_err() {
            tracing::warn!("Table Handler pending stream is full, dropping connection");
        }
    }

    fn on_upgrade_error(
        &mut self,
        _user_data: Self::InboundUserData,
        _error: <Self::InboundUpgrade as InboundUpgradeSend>::Error,
    ) {
        unreachable!("Unreachable Table Handler upgrade error")
    }
}

async fn make_join_table_request(
    io: Substream,
    mut authenticate: mpsc::Sender<(
        proto::JoinTableReq,
        oneshot::Sender<Result<SessionAccepted, common::Code>>,
    )>,
) -> Result<Option<JoinedSession>, io::Error> {
    let mut framed = Framed::new(
        io,
        ProtobufUviCodec::<proto::JoinTableReq>::new(MAX_MESSAGE_SIZE),
    );

    let join_request = framed
        .next()
        .await
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "stream closed"))??;

    let (tx, rx) = oneshot::channel();

    authenticate
        .send((join_request, tx))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "authentication handler dropped"))?;

    let result = rx
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "authentication response dropped"))?;

    let mut framed = Framed::from_parts(
        framed
            .into_parts()
            .map_codec(|_| ProtobufUviCodec::<proto::JoinTableResp>::new(MAX_MESSAGE_SIZE)),
    );

    match result {
        Ok(SessionAccepted {
            session_id,
            table_id,
            event_sender,
            event_receiver,
            table,
        }) => {
            framed
                .send(proto::JoinTableResp {
                    code: common::Code::Ok as i32,
                    table: Some(table),
                })
                .await?;

            framed.flush().await?;

            let framed = Framed::from_parts(framed.into_parts().map_codec(|_| {
                CombinedCodec::new(
                    ProtobufUviCodec::new(MAX_MESSAGE_SIZE),
                    ProtobufUviCodec::new(MAX_MESSAGE_SIZE),
                )
            }));
            Ok(Some(JoinedSession {
                io: framed,
                session_id,
                table_id,
                event_sender,
                event_receiver,
                pending_server_message: None,
            }))
        }
        Err(rejected) => {
            framed
                .send(proto::JoinTableResp {
                    code: rejected as i32,
                    table: None,
                })
                .await?;
            framed.flush().await?;
            Ok(None)
        }
    }
}

struct JoinedSession {
    io: Framed<
        Substream,
        CombinedCodec<
            ProtobufUviCodec<proto::ServerProtocol>,
            ProtobufUviCodec<proto::ClientProtocol>,
        >,
    >,
    session_id: SessionId,
    table_id: TableId,
    event_sender: mpsc::Sender<proto::ClientMessage>,
    event_receiver: mpsc::Receiver<proto::ServerMessage>,
    pending_server_message: Option<proto::ServerMessage>,
}

impl Future for JoinedSession {
    type Output = Result<(), io::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            // 写入服务器包
            if self.io.poll_ready_unpin(cx).is_ready() {
                if let Some(message) = self.pending_server_message.take() {
                    self.io.start_send_unpin(proto::ServerProtocol {
                        message: Some(message),
                    })?;
                    continue;
                }
            }
            // Flush 缓冲区
            match self.io.poll_flush_unpin(cx)? {
                Poll::Ready(()) => {}
                Poll::Pending => {}
            }

            match self.io.poll_next_unpin(cx)? {
                Poll::Ready(Some(proto::ClientProtocol {
                    message: Some(message),
                })) => {
                    self.event_sender.try_send(message).map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("event send error: {}", e))
                    })?;
                    continue;
                }
                Poll::Ready(Some(_)) => {
                    // TODO packet invalid!
                    continue;
                }
                Poll::Ready(None) => {
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => {}
            }
            if self.pending_server_message.is_none() {
                match self.event_receiver.poll_next_unpin(cx) {
                    Poll::Ready(Some(message)) => {
                        self.pending_server_message = Some(message);
                        continue;
                    }
                    Poll::Ready(None) => {
                        return Poll::Ready(Ok(()));
                    }
                    Poll::Pending => {}
                }
            }
            return Poll::Pending;
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Authenticate {
        request: proto::JoinTableReq,
        responder: oneshot::Sender<Result<SessionAccepted, common::Code>>,
    },
    JoinAccepted {
        session_id: SessionId,
        table_id: TableId,
    },
}
