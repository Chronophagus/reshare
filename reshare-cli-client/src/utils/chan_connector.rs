use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct ChanConnector<In, Out, F> {
    in_chan: mpsc::UnboundedReceiver<In>,
    out_chan: mpsc::Sender<Out>,
    adapter: F,
}

impl<In, Out, F> ChanConnector<In, Out, F>
where
    In: Send + 'static,
    Out: Send + 'static,
    F: Fn(In) -> Out + Send + 'static,
{
    pub fn connect_with(
        in_chan: mpsc::UnboundedReceiver<In>,
        out_chan: mpsc::Sender<Out>,
        adapter: F,
    ) -> Self {
        Self {
            in_chan,
            out_chan,
            adapter,
        }
    }

    pub fn seal(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(in_data) = self.in_chan.recv().await {
                let out_data = (self.adapter)(in_data);
                let _ = self.out_chan.send(out_data).await;
            }
        })
    }
}
