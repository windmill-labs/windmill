use futures::{Sink, Stream};
use tokio::{
    io::AsyncRead,
    process::{ChildStdin, ChildStdout},
};

use super::IntoTransport;
use crate::service::{RxJsonRpcMessage, ServiceRole, TxJsonRpcMessage};

pub(crate) fn child_process(
    mut child: tokio::process::Child,
) -> std::io::Result<(tokio::process::Child, (ChildStdout, ChildStdin))> {
    if child.stdin.is_none() {
        return Err(std::io::Error::other("std in was taken"));
    }
    if child.stdout.is_none() {
        return Err(std::io::Error::other("std out was taken"));
    }
    let child_stdin = child.stdin.take().expect("already checked");
    let child_stdout = child.stdout.take().expect("already checked");
    Ok((child, (child_stdout, child_stdin)))
}

pub struct TokioChildProcess {
    child: tokio::process::Child,
    child_stdin: ChildStdin,
    child_stdout: ChildStdout,
}

// we hold the child process with stdout, for it's easier to implement AsyncRead
pin_project_lite::pin_project! {
    pub struct TokioChildProcessOut {
        child: tokio::process::Child,
        #[pin]
        child_stdout: ChildStdout,
    }
}

impl AsyncRead for TokioChildProcessOut {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.project().child_stdout.poll_read(cx, buf)
    }
}

impl TokioChildProcess {
    pub fn new(child: &mut tokio::process::Command) -> std::io::Result<Self> {
        child
            .kill_on_drop(true)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped());
        let (child, (child_stdout, child_stdin)) = child_process(child.spawn()?)?;
        Ok(Self {
            child,
            child_stdin,
            child_stdout,
        })
    }

    pub fn split(self) -> (TokioChildProcessOut, ChildStdin) {
        let TokioChildProcess {
            child,
            child_stdin,
            child_stdout,
        } = self;
        (
            TokioChildProcessOut {
                child,
                child_stdout,
            },
            child_stdin,
        )
    }
}

impl<R: ServiceRole> IntoTransport<R, std::io::Error, ()> for TokioChildProcess {
    fn into_transport(
        self,
    ) -> (
        impl Sink<TxJsonRpcMessage<R>, Error = std::io::Error> + Send + 'static,
        impl Stream<Item = RxJsonRpcMessage<R>> + Send + 'static,
    ) {
        IntoTransport::<R, std::io::Error, super::io::TransportAdapterAsyncRW>::into_transport(
            self.split(),
        )
    }
}
