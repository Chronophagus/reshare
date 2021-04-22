use actix_web::body::SizedStream;
use actix_web::error::{BlockingError, Error as ActixError};
use actix_web::web::{self, Bytes, BytesMut};
use futures::Stream;
use reshare_models::FileInfo;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;

pub type Result<T, E = DownloadError> = std::result::Result<T, E>;

const MIN_BUF_SIZE_KB: usize = 4;
const MAX_BUF_SIZE_KB: usize = 8192 * 2;

pub async fn download_file_stream(
    file_info: &FileInfo,
) -> Result<SizedStream<impl Stream<Item = Result<Bytes, ActixError>>>> {
    let storage_path = file_info.storage_path.clone();
    let file = web::block(move || std::fs::File::open(storage_path.clone())).await?;

    let stream = DownloadStream::from(file);
    Ok(SizedStream::new(file_info.size, stream))
}

struct DownloadStream {
    state: DownloadState,
    read_multiplier: usize,
}

type PendingReadFutOutput = Result<(std::fs::File, BytesMut, usize), BlockingError<std::io::Error>>;

enum DownloadState {
    NewChunkAvailable(Option<std::fs::File>),
    PendingRead(Pin<Box<dyn Future<Output = PendingReadFutOutput>>>),
}

impl From<std::fs::File> for DownloadStream {
    fn from(file: std::fs::File) -> Self {
        Self {
            state: DownloadState::NewChunkAvailable(Some(file)),
            read_multiplier: MIN_BUF_SIZE_KB,
        }
    }
}

impl Stream for DownloadStream {
    type Item = Result<Bytes, ActixError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use std::io::prelude::*;
        const KB: usize = 1024;

        let this = self.get_mut();

        if let DownloadState::NewChunkAvailable(ref mut file) = this.state {
            let mut file = file.take().unwrap();

            let buf_size = this.read_multiplier * KB;
            let mut buf = BytesMut::with_capacity(buf_size);
            unsafe { buf.set_len(buf_size) }

            let fut = web::block(move || {
                let bytes_read = file.read(&mut buf)?;
                Ok::<_, std::io::Error>((file, buf, bytes_read))
            });

            this.state = DownloadState::PendingRead(Box::pin(fut))
        }

        let ret = match this.state {
            DownloadState::PendingRead(ref mut fut) => match fut.as_mut().poll(cx) {
                Poll::Ready(Ok((_, _, bytes_read))) if bytes_read == 0 => Poll::Ready(None),
                Poll::Ready(Ok((file, mut buf, bytes_read))) => {
                    this.state = DownloadState::NewChunkAvailable(Some(file));

                    if buf.len() == bytes_read {
                        if this.read_multiplier < MAX_BUF_SIZE_KB {
                            this.read_multiplier *= 2;
                        }
                    } else {
                        if this.read_multiplier > MIN_BUF_SIZE_KB {
                            this.read_multiplier /= 2;
                        }
                        buf.truncate(bytes_read);
                    }

                    Poll::Ready(Some(Ok(buf.freeze())))
                }
                Poll::Ready(Err(e)) => {
                    log::error!("Read error {}", e);
                    Poll::Ready(Some(Err(e.into())))
                }
                Poll::Pending => Poll::Pending,
            },
            _ => unreachable!(),
        };

        ret
    }
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("Error reading file")]
    FileReadError {
        #[from]
        source: BlockingError<std::io::Error>,
    },
}

impl actix_web::error::ResponseError for DownloadError {
    fn error_response(&self) -> actix_web::HttpResponse {
        use actix_web::dev::HttpResponseBuilder;

        HttpResponseBuilder::new(self.status_code()).json(reshare_models::Error {
            error_msg: self.to_string(),
        })
    }
}
