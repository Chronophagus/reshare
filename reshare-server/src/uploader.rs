use crate::multipart::{
    MultipartFields, MultipartFileChunk, MultipartFiles, MultipartProcessingError,
};
use actix_multipart::Multipart;
use actix_web::{error::BlockingError, web};
use futures::StreamExt;
use once_cell::sync::{Lazy, OnceCell};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

const SERVER_DIRECTORY_NAME: &str = "reshare_files";

pub type Result<T, E = UploadError> = std::result::Result<T, E>;

pub async fn save_file<S>(
    file_name: String,
    mut file_stream: impl std::convert::AsMut<S>,
) -> Result<reshare_models::FileInfo>
where
    S: StreamExt<Item = MultipartFileChunk> + Unpin,
{
    use rand::{distributions::Alphanumeric, Rng};

    let stream = file_stream.as_mut();
    let mut bytes_written: u64 = 0;

    let actual_name: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect();

    let storage_path = get_work_dir().join(&actual_name);

    let mut f = {
        let storage_path = storage_path.clone();
        web::block(|| std::fs::File::create(storage_path)).await?
    };

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let chunk_size = chunk.len();

        f = web::block(move || f.write_all(&chunk).map(|_| f)).await?;
        bytes_written += chunk_size as u64;
    }

    if bytes_written == 0 {
        {
            let storage_path = storage_path.clone();
            let _ = web::block(move || std::fs::remove_file(storage_path)).await;
        }

        Err(UploadError::EmptyFile)
    } else {
        Ok(reshare_models::FileInfo {
            name: file_name,
            size: bytes_written,
            upload_date: chrono::Local::now(),
            storage_path,
        })
    }
}

pub async fn cleanup() {
    let _ = web::block(|| std::fs::remove_dir_all(get_work_dir())).await;
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
        }
    }
}

impl From<BlockingError<std::io::Error>> for UploadError {
    fn from(err: BlockingError<std::io::Error>) -> UploadError {
        log::error!("{}", err);
        UploadError::InternalFailure
    }
}

fn get_work_dir() -> &'static Path {
    static DIR: OnceCell<PathBuf> = OnceCell::new();

    DIR.get_or_init(|| {
        let root_path = dirs_next::home_dir().unwrap_or(PathBuf::from("/"));
        let dir_path = root_path.join(SERVER_DIRECTORY_NAME);
        let _ = std::fs::create_dir(&dir_path);

        dir_path
    })
    .as_path()
}
