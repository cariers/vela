use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use asynchronous_codec::BytesMut;
use futures::{AsyncRead, AsyncWrite, Sink, Stream, ready};

/// Protobuf 底层IO
#[pin_project::pin_project]
pub struct Framed<TInput, TOutput, S>
where
    S: AsyncRead + AsyncWrite + Unpin,
    TInput: prost::Message,
    TOutput: prost::Message,
{
    #[pin]
    io: asynchronous_codec::Framed<S, unsigned_varint::codec::UviBytes>,
    write_buffer: BytesMut,
    _priv: PhantomData<(TInput, TOutput)>,
}

impl<I, O, S> Framed<I, O, S>
where
    I: prost::Message + Default,
    O: prost::Message + Default,
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(socket: S) -> Self {
        Self {
            io: asynchronous_codec::Framed::new(
                socket,
                unsigned_varint::codec::UviBytes::default(),
            ),
            write_buffer: BytesMut::new(),
            _priv: PhantomData,
        }
    }

    pub fn new_framed(io: asynchronous_codec::Framed<S, unsigned_varint::codec::UviBytes>) -> Self {
        Self {
            io,
            write_buffer: BytesMut::new(),
            _priv: PhantomData,
        }
    }
}

impl<I, O, S> Stream for Framed<I, O, S>
where
    I: prost::Message + Default,
    O: prost::Message + Default,
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<I, FrameError>;

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

impl<I, O, S> Sink<O> for Framed<I, O, S>
where
    I: prost::Message + Default,
    O: prost::Message + Default,
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Error = FrameError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.io.poll_ready(cx).map_err(FrameError::Io)
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
        this.io.poll_flush(cx).map_err(FrameError::Io)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.io.poll_close(cx).map_err(FrameError::Io)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("Protobuf encode error: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("Stream is closed")]
    Closed,
}
