// 引用Protobuf
include!(concat!(env!("OUT_DIR"), "/vela.table.rs"));

use std::{
    io, iter,
    pin::Pin,
    task::{Context, Poll},
};

use asynchronous_codec::{BytesMut, Framed, LengthCodec};
use futures::{
    AsyncRead, AsyncReadExt, AsyncWrite, FutureExt, Sink, SinkExt, Stream, StreamExt, ready,
};
use prost::Message;
use vela_core::protobuf;
use volans::{
    core::{InboundUpgrade, UpgradeInfo},
    swarm::{InboundStreamHandler, StreamProtocol, Substream, connection::InboundConnection},
};

pub const PROTOCOL_NAME: StreamProtocol = StreamProtocol::new("/v1/table");

pub type IoStream = protobuf::IoStream<IncomingMessage, OutgoingMessage, Substream>;

// pub struct Upgrade;

// impl UpgradeInfo for Upgrade {
//     type Info = StreamProtocol;
//     type InfoIter = iter::Once<Self::Info>;

//     fn protocol_info(&self) -> Self::InfoIter {
//         iter::once(PROTOCOL_NAME)
//     }
// }

// impl<S> InboundUpgrade<S> for Upgrade
// where
//     S: AsyncWrite + AsyncRead + Unpin + 'static + Send,
// {
//     type Error = io::Error;
//     type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;
//     type Output = ([u8; 64], SocketStream<S>);

//     fn upgrade_inbound(self, mut socket: S, _info: Self::Info) -> Self::Future {
//         // 读取table id
//         async move {
//             let mut buffer = [0_u8; 64];
//             socket.read_exact(&mut buffer).await?;
//             Ok((buffer, SocketStream::new(socket)))
//         }
//         .boxed()
//     }
// }
