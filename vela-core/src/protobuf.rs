use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use asynchronous_codec::{Framed, LengthCodec};
use futures::{AsyncRead, AsyncWrite, Sink, Stream, ready};
use prost::bytes::BytesMut;

/// Protobuf 底层IO
#[pin_project::pin_project]
pub struct IoStream<I, O, S>
where
    S: AsyncRead + AsyncWrite + Unpin,
    I: prost::Message,
    O: prost::Message,
{
    #[pin]
    io: Framed<S, LengthCodec>,
    write_buffer: BytesMut,
    _priv: PhantomData<(I, O)>,
}

impl<I, O, S> Stream for IoStream<I, O, S>
where
    I: prost::Message + Default,
    O: prost::Message + Default,
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<I, StreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match ready!(this.io.poll_next(cx)?) {
            None => Poll::Ready(None),
            Some(bytes) => {
                let message = I::decode(bytes.as_ref())?;
                Poll::Ready(Some(Ok(message)))
            }
        }
    }
}

impl<I, O, S> Sink<O> for IoStream<I, O, S>
where
    I: prost::Message + Default,
    O: prost::Message + Default,
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Error = StreamError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.io.poll_ready(cx).map_err(StreamError::Io)
    }

    fn start_send(self: Pin<&mut Self>, item: O) -> Result<(), Self::Error> {
        let this = self.project();
        item.encode(&mut this.write_buffer.as_mut())?;
        let buffer = this.write_buffer.split().freeze();
        this.io.start_send(buffer)?;
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.io.poll_flush(cx).map_err(StreamError::Io)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.io.poll_close(cx).map_err(StreamError::Io)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("Protobuf encode error: {0}")]
    Encode(#[from] prost::EncodeError),
}
