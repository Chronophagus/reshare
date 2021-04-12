use bytes::Buf;
use futures::Stream;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

pub type BytesTransmitted = u64;
pub type Monitor = mpsc::UnboundedReceiver<BytesTransmitted>;
type Reporter = mpsc::UnboundedSender<BytesTransmitted>;

#[pin_project]
pub struct MonitoredStream<S> {
    #[pin]
    stream: S,
    reporter: Reporter,
}

impl<S: Stream> MonitoredStream<S> {
    pub fn new(stream: S) -> (Self, Monitor) {
        let (reporter, monitor) = mpsc::unbounded_channel();

        (Self { stream, reporter }, monitor)
    }
}

impl<S, B, E> Stream for MonitoredStream<S>
where
    S: Stream<Item = Result<B, E>>,
    B: Buf,
    E: std::error::Error,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.stream.poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                let _ = this.reporter.send(bytes.remaining() as u64);
                Poll::Ready(Some(Ok(bytes)))
            }
            poll => poll,
        }
    }
}
