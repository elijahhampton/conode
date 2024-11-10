use async_trait::async_trait;
use bincode;
use libp2p::{
    futures::AsyncRead, futures::AsyncReadExt, futures::AsyncWrite, futures::AsyncWriteExt,
    request_response::Codec,
};
use std::io;

use conode_types::negotiation::{NegotiationRequest, NegotiationResponse};

/// Protocol identifier for the negotiation request/response protocol
pub const NEGOTATION_REQUEST_RESPONSE_PROTOCOL: &str = "/conode/negotiation/1.0.0";

/// Codec implementation for serializing and deserializing negotiation messages
/// Uses bincode for efficient binary serialization
#[derive(Debug, Clone, Default)]
pub struct NegotiationCodec;

#[async_trait]
impl Codec for NegotiationCodec {
    type Protocol = &'static str;
    type Request = NegotiationRequest;
    type Response = NegotiationResponse;

    /// Reads and deserializes a negotiation request from the network stream
    ///
    /// # Format
    /// - 4 bytes: length prefix (big endian u32)
    /// - Remaining bytes: bincode serialized NegotiationRequest
    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut length_bytes = [0u8; 4];
        io.read_exact(&mut length_bytes).await?;
        let length = u32::from_be_bytes(length_bytes) as usize;

        let mut buffer = vec![0u8; length];
        io.read_exact(&mut buffer).await?;

        bincode::deserialize(&buffer).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Reads and deserializes a negotiation response from the network stream
    ///
    /// # Format
    /// - 4 bytes: length prefix (big endian u32)
    /// - Remaining bytes: bincode serialized NegotiationResponse
    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut length_bytes = [0u8; 4];
        io.read_exact(&mut length_bytes).await?;
        let length = u32::from_be_bytes(length_bytes) as usize;

        let mut buffer = vec![0u8; length];
        io.read_exact(&mut buffer).await?;

        bincode::deserialize(&buffer).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Serializes and writes a negotiation request to the network stream
    ///
    /// # Format
    /// - 4 bytes: length prefix (big endian u32)
    /// - Remaining bytes: bincode serialized NegotiationRequest
    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let buffer =
            bincode::serialize(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let length = buffer.len() as u32;
        io.write_all(&length.to_be_bytes()).await?;
        io.write_all(&buffer).await
    }

    /// Serializes and writes a negotiation response to the network stream
    ///
    /// # Format
    /// - 4 bytes: length prefix (big endian u32)
    /// - Remaining bytes: bincode serialized NegotiationResponse
    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let buffer =
            bincode::serialize(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let length = buffer.len() as u32;
        io.write_all(&length.to_be_bytes()).await?;
        io.write_all(&buffer).await
    }
}
