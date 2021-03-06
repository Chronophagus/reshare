pub mod chan_connector;
pub mod monitored_stream;
pub mod progress_tracker;

pub use chan_connector::ChanConnector;
pub use monitored_stream::MonitoredStream;

pub trait OptionExt<T> {
    fn ok_or_try<F, E>(self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_try<F, E>(self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        match self {
            Some(v) => Ok(v),
            None => f(),
        }
    }
}
