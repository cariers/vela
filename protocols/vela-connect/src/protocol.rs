// 引用Protobuf
include!(concat!(env!("OUT_DIR"), "/volans.connect.rs"));

use std::io;

use asynchronous_codec::{Framed, LengthCodec};
use futures::{
    SinkExt, StreamExt,
    io::{AsyncRead, AsyncWrite},
};
use prost::{Message, bytes::BytesMut};
use volans::swarm::StreamProtocol;

pub const PROTOCOL_NAME: StreamProtocol = StreamProtocol::new("/v1/connect");

pub async fn read_connection_info<S>(stream: S, server_info: ServerInfo) -> io::Result<ConnectInfo>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut stream = Framed::new(stream, LengthCodec);
    tracing::debug!("Receiving connect info: {:?}", server_info);
    let buffer = stream.next().await.unwrap_or_else(|| {
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Connection closed before reading",
        ))
    })?;
    let connect_info = ConnectInfo::decode(&buffer[..])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    tracing::info!("Received connect info: {:?}", connect_info);

    let mut send_buffer = BytesMut::new();

    server_info
        .encode(&mut send_buffer)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    stream.send(send_buffer.freeze()).await?;
    stream.flush().await?;
    stream.close().await?;
    Ok(connect_info)
}

pub async fn read_server_info<S>(stream: S, connect_info: ConnectInfo) -> io::Result<ServerInfo>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut stream = Framed::new(stream, LengthCodec);

    let mut send_buffer = BytesMut::new();

    connect_info
        .encode(&mut send_buffer)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    tracing::debug!("Sending connect info: {:?}", connect_info);

    stream.send(send_buffer.freeze()).await?;
    stream.flush().await?;
    let buffer = stream.next().await.unwrap_or_else(|| {
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Connection closed before reading",
        ))
    })?;
    let server_info = ServerInfo::decode(&buffer[..])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    tracing::info!("Received server info: {:?}", server_info);
    stream.close().await?;
    Ok(server_info)
}
