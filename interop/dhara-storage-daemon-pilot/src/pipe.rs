//! Named-pipe transport helpers for tonic on Windows.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_stream::stream;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tonic::transport::server::{Connected, Router};
use tracing::{debug, info};

/// Default pipe path used when the harness does not pass an override.
pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\dhara-storage-pilot";

/// Wrap a connected named pipe so tonic can treat it as an HTTP/2 transport.
pub struct PipeConnection {
    inner: NamedPipeServer,
}

impl Connected for PipeConnection {
    type ConnectInfo = ();

    fn connect_info(&self) -> Self::ConnectInfo {}
}

impl AsyncRead for PipeConnection {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for PipeConnection {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Accept named-pipe clients and serve the provided tonic router until the last client disconnects.
pub async fn serve_named_pipe(pipe_name: &str, router: Router) -> Result<(), Box<dyn std::error::Error>> {
    let pipe_name = pipe_name.to_string();
    let incoming = stream! {
        let mut first = true;
        loop {
            let server = match ServerOptions::new()
                .first_pipe_instance(first)
                .create(&pipe_name)
            {
                Ok(server) => server,
                Err(err) => {
                    yield Err(err);
                    break;
                }
            };
            first = false;

            debug!(pipe = %pipe_name, "waiting for named-pipe client");
            if let Err(err) = server.connect().await {
                yield Err(err);
                break;
            }

            info!(pipe = %pipe_name, "named-pipe client connected");
            yield Ok(PipeConnection { inner: server });
        }
    };

    router.serve_with_incoming(incoming).await?;
    Ok(())
}
