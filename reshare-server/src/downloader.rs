use actix_web::body::SizedStream;
use actix_web::error::{BlockingError, Error as ActixError};
use actix_web::web::{self, Bytes};
use futures::Stream;
use reshare_models::FileInfo;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;

pub type Result<T, E = DownloadError> = std::result::Result<T, E>;

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
}

enum DownloadState {
    NewChunkAvailable(Option<std::fs::File>),
    PendingRead(
        Pin<
            Box<
                dyn Future<
                    Output = Result<(std::fs::File, Vec<u8>, usize), BlockingError<std::io::Error>>,
                >,
            >,
        >,
    ),
}

impl From<std::fs::File> for DownloadStream {
    fn from(file: std::fs::File) -> Self {
        Self {
            state: DownloadState::NewChunkAvailable(Some(file)),
        }
    }
}

impl Stream for DownloadStream {
    type Item = Result<Bytes, ActixError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use std::io::prelude::*;
        const BUF_SIZE: usize = 8 * 1024;

        let this = self.get_mut();

        if let DownloadState::NewChunkAvailable(ref mut file) = this.state {
            let mut file = file.take().unwrap();
            let mut buf = Vec::with_capacity(BUF_SIZE);
            unsafe { buf.set_len(BUF_SIZE) }

            let fut = web::block(move || {
                let bytes_read = file.read(&mut buf)?;
                Ok::<_, std::io::Error>((file, buf, bytes_read))
            });

            this.state = DownloadState::PendingRead(Box::pin(fut));
        }

        let ret = if let DownloadState::PendingRead(ref mut fut) = this.state {
            match fut.as_mut().poll(cx) {
                Poll::Ready(Ok((file, mut buf, bytes_read))) => {
                    log::debug!("Read {} bytes", bytes_read);
                    this.state = DownloadState::NewChunkAvailable(Some(file));

                    if bytes_read > 0 {
                        buf.truncate(bytes_read);
                        Poll::Ready(Some(Ok(Bytes::from(buf))))
                    } else {
                        Poll::Ready(None)
                    }
                }
                Poll::Ready(Err(e)) => {
                    log::debug!("Read error");
                    Poll::Ready(Some(Err(e.into())))
                }
                Poll::Pending => {
                    log::debug!("Pending read");
                    Poll::Pending
                }
            }
        } else {
            unreachable!();
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
