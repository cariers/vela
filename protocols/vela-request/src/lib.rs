pub mod client;
pub mod server;

use std::io;

use async_trait::async_trait;
use futures::{AsyncRead, AsyncWrite};
use vela_protobuf::common;
use volans::{
    core::PeerId,
    request::{self, codec::ProtobufCodec},
    swarm::{ConnectionId, StreamProtocol},
};

pub use volans::request::{Config, InboundFailure, OutboundFailure, RequestId};

#[derive(Debug)]
pub struct Request<B> {
    service: String,
    metadata: Vec<common::Metadata>,
    payload: B,
}

impl<B> Request<B> {
    pub fn new(service: String, payload: B) -> Self {
        Self {
            service,
            metadata: Vec::new(),
            payload,
        }
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    pub fn metadata(&self) -> &Vec<common::Metadata> {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut Vec<common::Metadata> {
        &mut self.metadata
    }

    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.push(common::Metadata { key, value });
    }

    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata
            .iter()
            .find(|m| m.key == key)
            .map(|m| &m.value)
    }

    pub fn payload(&self) -> &B {
        &self.payload
    }

    pub fn into_payload(self) -> B {
        self.payload
    }
}

#[derive(Debug)]
pub struct Response<B> {
    metadata: Vec<common::Metadata>,
    payload: Result<B, common::Status>,
}

#[derive(Clone)]
pub struct Codec<TInput, TOutput> {
    inner: ProtobufCodec<common::Request, common::Response>,
    _marker: std::marker::PhantomData<(TInput, TOutput)>,
}

impl<TInput, TOutput> Codec<TInput, TOutput>
where
    TInput: prost::Message + Default + Send,
    TOutput: prost::Message + Default + Send,
{
    pub fn new() -> Self {
        Self {
            inner: ProtobufCodec::new(),
            _marker: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<TInput, TOutput> request::Codec for Codec<TInput, TOutput>
where
    TInput: prost::Message + Default + Send,
    TOutput: prost::Message + Default + Send,
{
    type Protocol = StreamProtocol;
    type Request = Request<TInput>;
    type Response = Response<TOutput>;

    async fn read_request<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let common_request = self.inner.read_request(protocol, io).await?;
        Ok(Request {
            service: common_request.service,
            metadata: common_request.metadata,
            payload: TInput::decode(common_request.payload.as_slice())?,
        })
    }
    async fn read_response<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let common_response = self.inner.read_response(protocol, io).await?;
        let status = common_response.status.unwrap_or_default();
        if status.code == common::Code::Ok as i32 {
            return Ok(Response {
                metadata: common_response.metadata,
                payload: Ok(TOutput::decode(common_response.payload.as_slice())?),
            });
        } else {
            return Ok(Response {
                metadata: common_response.metadata,
                payload: Err(status),
            });
        }
    }
    async fn write_request<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
        request: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let common_request = common::Request {
            service: request.service,
            metadata: request.metadata,
            payload: request.payload.encode_to_vec(),
        };
        self.inner.write_request(protocol, io, common_request).await
    }
    async fn write_response<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
        response: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        match response.payload {
            Ok(payload) => {
                let common_response = common::Response {
                    status: Some(common::Status::default()),
                    metadata: response.metadata,
                    payload: payload.encode_to_vec(),
                };
                self.inner
                    .write_response(protocol, io, common_response)
                    .await
            }
            Err(status) => {
                let common_response = common::Response {
                    status: Some(status),
                    metadata: response.metadata,
                    payload: Vec::new(), // Empty payload for error responses
                };
                self.inner
                    .write_response(protocol, io, common_response)
                    .await
            }
        }
    }
}

#[derive(Debug)]
pub struct Responder<TResponse> {
    metadata: Vec<common::Metadata>,
    tx: request::Responder<Response<TResponse>>,
}

impl<TResponse> Responder<TResponse> {
    pub(crate) fn new(tx: request::Responder<Response<TResponse>>) -> Self {
        Self {
            metadata: Vec::new(),
            tx,
        }
    }

    pub fn metadata(&self) -> &Vec<common::Metadata> {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut Vec<common::Metadata> {
        &mut self.metadata
    }

    pub fn add_metadata(&mut self, metadata: common::Metadata) {
        self.metadata.push(metadata);
    }

    pub fn add_metadata_from_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = common::Metadata>,
    {
        self.metadata.extend(iter);
    }

    pub fn send_response(
        self,
        result: Result<TResponse, common::Status>,
    ) -> Result<(), Result<TResponse, common::Status>> {
        let response = Response {
            metadata: self.metadata,
            payload: result,
        };
        self.tx.send_response(response).map_err(|r| r.payload)
    }

    pub fn ok_response(self, payload: TResponse) -> Result<(), Result<TResponse, common::Status>> {
        self.send_response(Ok(payload))
    }

    pub fn err_response(
        self,
        status: common::Status,
    ) -> Result<(), Result<TResponse, common::Status>> {
        self.send_response(Err(status))
    }
}
