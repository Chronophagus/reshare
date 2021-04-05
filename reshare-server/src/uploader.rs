use crate::multipart::{
    MultipartFields, MultipartFileChunk, MultipartFiles, MultipartProcessingError,
};
use actix_multipart::Multipart;
use actix_web::{error::BlockingError, web};
use futures::StreamExt;
use once_cell::sync::Lazy;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};
use thiserror::Error;

pub type Result<T, E = UploadError> = std::result::Result<T, E>;

static FILES_TO_REMOVE: Lazy<RemovalQueue> = Lazy::new(|| RemovalQueue::new());

pub async fn save_file<S>(
    file_name: String,
    mut file_stream: impl std::convert::AsMut<S>,
) -> Result<reshare_models::FileInfo>
where
    S: StreamExt<Item = MultipartFileChunk> + Unpin,
{
    use rand::{distributions::Alphanumeric, Rng};

    // remove scheduled for removal files
    let _ = web::block(move || {
        let files = FILES_TO_REMOVE.clone();
        let mut queue = files.queue.lock().unwrap();

        for file_info in queue.iter() {
            let _ = std::fs::remove_file(&file_info.storage_path);
        }

        queue.clear();

        Ok::<(), ()>(())
    })
    .await;

    let stream = file_stream.as_mut();
    let mut bytes_written: u64 = 0;

    let actual_name: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect();

    let mut f = {
        let actual_name = actual_name.clone();
        web::block(|| std::fs::File::create(actual_name)).await?
    };

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let chunk_size = chunk.len();

        f = web::block(move || f.write_all(&chunk).map(|_| f)).await?;
        bytes_written += chunk_size as u64;
    }

    log::debug!("File size: {}", bytes_written);

    if bytes_written == 0 {
        let _ = web::block(move || std::fs::remove_file(actual_name)).await;
        Err(UploadError::EmptyFile)
    } else {
        Ok(reshare_models::FileInfo {
            name: file_name,
            size: bytes_written,
            upload_date: chrono::Local::now(),
            storage_path: std::path::PathBuf::from(actual_name),
        })
    }
}

pub fn schedule_removal(file_info: reshare_models::FileInfo) {
    FILES_TO_REMOVE.add(file_info);
}

pub struct UploadForm {
    pub keyphrase: Option<String>,
    pub files: MultipartFiles,
}

impl UploadForm {
    pub async fn try_from_multipart(form_data: Multipart) -> Result<UploadForm> {
        let mut fields = MultipartFields::from(form_data);

        let keyphrase = fields
            .next_text_field("keyphrase")
            .await?
            .filter(|s| !s.is_empty());

        Ok(UploadForm {
            keyphrase,
            files: fields.parse_files(),
        })
    }
}

#[derive(Debug, Error)]
pub enum UploadError {
    #[error("Error processing multipart data")]
    Multipart {
        #[from]
        source: MultipartProcessingError,
    },

    #[error("Empty files not allowed")]
    EmptyFile,

    #[error("Operation failed due to internal failure")]
    InternalFailure,

    #[error("File exists")]
    FileExists(reshare_models::FileInfo),
}

impl actix_web::error::ResponseError for UploadError {
    fn error_response(&self) -> actix_web::HttpResponse {
        use actix_web::dev::HttpResponseBuilder;

        HttpResponseBuilder::new(self.status_code()).json(reshare_models::Error {
            error_msg: self.to_string(),
        })
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;
        match self {
            Self::Multipart { source: err } => err.status_code(),
            Self::EmptyFile => StatusCode::BAD_REQUEST,
            Self::InternalFailure => StatusCode::INTERNAL_SERVER_ERROR,
            Self::FileExists(_) => StatusCode::BAD_REQUEST,
        }
    }
}

impl From<BlockingError<std::io::Error>> for UploadError {
    fn from(err: BlockingError<std::io::Error>) -> UploadError {
        log::error!("{}", err);
        UploadError::InternalFailure
    }
}

#[derive(Clone)]
struct RemovalQueue {
    queue: Arc<Mutex<Vec<reshare_models::FileInfo>>>,
}

impl RemovalQueue {
    fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn add(&self, file_info: reshare_models::FileInfo) {
        let mut guard = self.queue.lock().unwrap();
        guard.push(file_info);
    }
}
