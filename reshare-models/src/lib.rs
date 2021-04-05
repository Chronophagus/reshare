pub mod error;
pub mod file_info;

pub use error::Error;
pub use file_info::FileInfo;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum FileUploadStatus {
    Success(FileInfo),
    Error(Error),
}

impl<E: std::error::Error> From<Result<FileInfo, E>> for FileUploadStatus {
    fn from(res: Result<FileInfo, E>) -> FileUploadStatus {
        match res {
            Ok(file_info) => Self::Success(file_info),
            Err(e) => Self::Error(Error {
                error_msg: e.to_string(),
            }),
        }
    }
}
