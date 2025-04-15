//! # Transport
//! The transport type must implemented [`IntoTransport`] trait, which allow split into a sink and a stream.
//!
//! For client, the sink item is [`ClientJsonRpcMessage`](crate::model::ClientJsonRpcMessage) and stream item is [`ServerJsonRpcMessage`](crate::model::ServerJsonRpcMessage)
//!
//! For server, the sink item is [`ServerJsonRpcMessage`](crate::model::ServerJsonRpcMessage) and stream item is [`ClientJsonRpcMessage`](crate::model::ClientJsonRpcMessage)
//!
//! ## These types is automatically implemented [`IntoTransport`] trait
//! 1. For type that already implement both [`Sink`] and [`Stream`] trait, they are automatically implemented [`IntoTransport`] trait
//! 2. For tuple of sink `Tx` and stream `Rx`, type `(Tx, Rx)` are automatically implemented [`IntoTransport`] trait
//! 3. For type that implement both [`tokio::io::AsyncRead`] and [`tokio::io::AsyncWrite`] trait, they are automatically implemented [`IntoTransport`] trait
//! 4. For tuple of [`tokio::io::AsyncRead`] `R `and [`tokio::io::AsyncWrite`] `W`, type `(R, W)` are automatically implemented [`IntoTransport`] trait
//!
//! ## Examples
//!
//! ```rust
//! # use rmcp::{
//! #     ServiceExt, serve_client, serve_server,
//! # };
//!
//! // create transport from tcp stream
//! async fn client() -> Result<(), Box<dyn std::error::Error>> {
//!     let stream = tokio::net::TcpSocket::new_v4()?
//!         .connect("127.0.0.1:8001".parse()?)
//!         .await?;
//!     let client = ().serve(stream).await?;
//!     let tools = client.peer().list_tools(Default::default()).await?;
//!     println!("{:?}", tools);
//!     Ok(())
//! }
//!
//! // create transport from std io
//! async fn io()  -> Result<(), Box<dyn std::error::Error>> {
//!     let client = None.serve((tokio::io::stdin(), tokio::io::stdout())).await?;
//!     let tools = client.peer().list_tools(Default::default()).await?;
//!     println!("{:?}", tools);
//!     Ok(())
//! }
//! ```

use futures::{Sink, Stream};

use crate::service::{RxJsonRpcMessage, ServiceRole, TxJsonRpcMessage};
#[cfg(feature = "transport-child-process")]
pub mod child_process;
#[cfg(feature = "transport-child-process")]
pub use child_process::TokioChildProcess;

#[cfg(feature = "transport-async-rw")]
pub mod io;
#[cfg(feature = "transport-io")]
pub use io::stdio;

#[cfg(feature = "transport-sse")]
pub mod sse;
#[cfg(feature = "transport-sse")]
pub use sse::SseTransport;

// #[cfg(feature = "tower")]
// pub mod tower;

#[cfg(feature = "transport-sse-server")]
pub mod sse_server;
#[cfg(feature = "transport-sse-server")]
pub use sse_server::SseServer;

// #[cfg(feature = "transport-ws")]
// pub mod ws;

pub trait IntoTransport<R, E, A>: Send + 'static
where
    R: ServiceRole,
    E: std::error::Error + Send + 'static,
{
    fn into_transport(
        self,
    ) -> (
        impl Sink<TxJsonRpcMessage<R>, Error = E> + Send + 'static,
        impl Stream<Item = RxJsonRpcMessage<R>> + Send + 'static,
    );
}

pub enum TransportAdapterStreamSink {}

impl<Role, Rx, Tx, E> IntoTransport<Role, E, TransportAdapterStreamSink> for (Tx, Rx)
where
    Role: ServiceRole,
    Tx: Sink<TxJsonRpcMessage<Role>, Error = E> + Send + 'static,
    Rx: Stream<Item = RxJsonRpcMessage<Role>> + Send + 'static,
    E: std::error::Error + Send + 'static,
{
    fn into_transport(
        self,
    ) -> (
        impl Sink<TxJsonRpcMessage<Role>, Error = E> + Send + 'static,
        impl Stream<Item = RxJsonRpcMessage<Role>> + Send + 'static,
    ) {
        self
    }
}

impl<R, T, E> IntoTransport<R, E, ()> for T
where
    T: Stream<Item = RxJsonRpcMessage<R>> + Sink<TxJsonRpcMessage<R>, Error = E> + Send + 'static,
    R: ServiceRole,
    E: std::error::Error + Send + 'static,
{
    fn into_transport(
        self,
    ) -> (
        impl Sink<TxJsonRpcMessage<R>, Error = E> + Send + 'static,
        impl Stream<Item = RxJsonRpcMessage<R>> + Send + 'static,
    ) {
        use futures::StreamExt;
        self.split()
    }
}
