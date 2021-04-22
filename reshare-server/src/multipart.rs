//! Helper utils to deal with multipart/form-data
//!

use actix_multipart::{Multipart, MultipartError};
use actix_web::{
    dev::HttpResponseBuilder, error::ResponseError, http::StatusCode, web::Bytes, HttpResponse,
};
use futures::{StreamExt, TryStreamExt};
use futures_core::Stream;
use thiserror::Error;

pub type Result<T, E = MultipartProcessingError> = std::result::Result<T, E>;
pub type MultipartFileChunk = Result<Bytes>;

pub struct MultipartFields {
    fields: Multipart,
}

impl From<Multipart> for MultipartFields {
    fn from(form_data: Multipart) -> Self {
        Self { fields: form_data }
    }
}

impl MultipartFields {
    pub fn parse_files(self) -> MultipartFiles {
        MultipartFiles::from(self.fields)
    }

    pub async fn next_text_field(&mut self, expected_field_name: &str) -> Result<Option<String>> {
        match self.fields.try_next().await {
            Ok(Some(mut field)) => {
                if field
                    .content_disposition()
                    .as_ref()
                    .and_then(|meta| meta.get_name())
                    .filter(|name| *name == expected_field_name)
                    .is_some()
                {
                    let mut buf = String::with_capacity(64);

                    while let Some(chunk) = field.next().await {
                        let string = chunk
                            .map_err(|e| MultipartProcessingError::FieldError { source: e })
                            .and_then(|bytes| {
                                std::str::from_utf8(bytes.as_ref())
                                    .map(|s| s.to_owned())
                                    .map_err(|_| MultipartProcessingError::InvalidField {
                                        name: expected_field_name.to_string(),
                                    })
                            })?;

                        buf.push_str(&string);
                    }

                    Ok(Some(buf))
                } else {
                    Err(MultipartProcessingError::InvalidField {
                        name: expected_field_name.to_owned(),
                    })
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(MultipartProcessingError::FieldError { source: e }),
        }
    }
}

pub struct MultipartFiles {
    files: Multipart,
}

impl From<Multipart> for MultipartFiles {
    fn from(form_data: Multipart) -> Self {
        Self { files: form_data }
    }
}

impl MultipartFiles {
    pub async fn next_file(
        &mut self,
    ) -> Result<Option<MultipartFile<impl StreamExt<Item = MultipartFileChunk>>>> {
        match self.files.try_next().await {
            Ok(Some(field)) => {
                let filename = field
                    .content_disposition()
                    .as_ref()
                    .and_then(|content| content.get_filename())
                    .filter(|&name| !name.is_empty())
                    .map(|s| {
                        let pos = s.rfind('/').unwrap_or(0);
                        s[pos..].to_owned()
                    })
                    .ok_or(MultipartProcessingError::InvalidFile)?;

                Ok(Some(MultipartFile {
                    filename,
                    file_stream: StreamMap::new(field.map(|res| {
                        res.map_err(|e| MultipartProcessingError::FileTransmissionError {
                            source: e,
                        })
                    })),
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(MultipartProcessingError::FileError { source: e }),
        }
    }
}

pub struct MultipartFile<S>
where
    S: StreamExt<Item = MultipartFileChunk>,
{
    pub filename: String,
    pub file_stream: StreamMap<S>,
}

// This wrapper is used to resolve generic type of closure in the next_file method
pub struct StreamMap<S> {
    stream: S,
}

impl<S, I> StreamMap<S>
where
    S: Stream<Item = I> + StreamExt<Item = I>,
{
    fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<S, I> std::ops::Deref for StreamMap<S>
where
    S: Stream<Item = I> + StreamExt<Item = I>,
{
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl<S, I> std::ops::DerefMut for StreamMap<S>
where
    S: Stream<Item = I> + StreamExt<Item = I>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}

impl<S, I> std::convert::AsRef<S> for StreamMap<S>
where
    S: Stream<Item = I> + StreamExt<Item = I>,
{
    fn as_ref(&self) -> &S {
        &self.stream
    }
}

impl<S, I> std::convert::AsMut<S> for StreamMap<S>
where
    S: Stream<Item = I> + StreamExt<Item = I>,
{
    fn as_mut(&mut self) -> &mut S {
        &mut self.stream
    }
}

#[derive(Debug, Error)]
pub enum MultipartProcessingError {
    #[error("Expected field {} is abscent", name)]
    InvalidField { name: String },

    #[error("Error parsing field multipart data")]
    FieldError { source: MultipartError },

    #[error("Error parsing file multipart data")]
    FileError { source: MultipartError },

    #[error("Invalid form input. Exected file")]
    InvalidFile,

    #[error("File transmission error")]
    FileTransmissionError { source: MultipartError },
}

impl ResponseError for MultipartProcessingError {
    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code()).json(reshare_models::Error {
            error_msg: self.to_string(),
        })
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}
