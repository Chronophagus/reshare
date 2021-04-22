use super::*;
use crate::utils::{
    progress_tracker::{ProgressReporter, ProgressTracker, ProgressUpdate},
    ChanConnector, MonitoredStream,
};
use anyhow::{anyhow, bail};
use futures::future;
use reqwest::{
    multipart::{Form, Part},
    Body,
};
use reshare_models::FileUploadStatus;
use std::{
    convert::{TryFrom, TryInto},
    path::PathBuf,
};
use tokio::{fs::File, runtime as rt};
use tokio_util::codec::{BytesCodec, FramedRead};

pub fn execute(args: PutArgs) -> Result<()> {
    let server_url = load_configuration()?;

    let files: Vec<FileRef> = args
        .file_list
        .into_iter()
        .filter_map(|path| path.try_into().ok())
        .collect();

    if files.is_empty() {
        bail!("No files to upload");
    }

    let query_url = server_url.join("api/")?.join("upload")?;
    let key_phrase = args.key_phrase;

    let mut upload_tracker = ProgressTracker::new();

    for file_ref in &files {
        upload_tracker.add_bar(file_ref.name.clone(), file_ref.len);
    }

    let upload_tasks: Vec<_> = files
        .iter()
        .map(|file_ref| {
            file_upload_task(
                query_url.clone(),
                file_ref.clone(),
                key_phrase.clone(),
                upload_tracker.get_reporter(),
            )
        })
        .collect();

    let rt = rt::Runtime::new()?;

    let results = rt.block_on(async move {
        let upload_tracking_task = upload_tracker.spawn();

        let results = future::join_all(upload_tasks).await;
        let _ = upload_tracking_task.await;
        results
    });

    for (res, file) in results.iter().zip(files.iter()) {
        match res {
            Ok(FileUploadStatus::Error(error_msg)) => {
                println!("{} - Error while uploading file: {}", file.name, error_msg);
            }
            Err(e) => println!("{} - operation failed. {}", file.name, e),
            _ => (),
        }
    }

    Ok(())
}

async fn file_upload_task(
    url: Url,
    file_ref: FileRef,
    keyphrase: Option<String>,
    progress_reporter: ProgressReporter,
) -> Result<FileUploadStatus> {
    let file = File::open(file_ref.path).await?;

    let file_stream = FramedRead::new(file, BytesCodec::new());
    let (file_stream, monitor) = MonitoredStream::new(file_stream);

    let file_name = file_ref.name.clone();

    ChanConnector::connect_with(monitor, progress_reporter, move |bytes_read| {
        ProgressUpdate {
            file_name: file_name.clone(),
            bytes_transmitted: bytes_read,
        }
    })
    .seal();

    let file_part = Part::stream_with_length(Body::wrap_stream(file_stream), file_ref.len)
        .file_name(file_ref.name.clone());

    let form = Form::new()
        .text("keyphrase", keyphrase.unwrap_or_default())
        .part("file", file_part);

    let response = reqwest::Client::new()
        .post(url)
        .multipart(form)
        .send()
        .await?
        .json::<Vec<FileUploadStatus>>()
        .await?;

    // There will be only 1 element in the vec
    // as we send only 1 file per task
    response
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Unexpected response from the server"))
}

#[derive(Debug, Clone)]
struct FileRef {
    name: String,
    len: u64,
    path: PathBuf,
}

impl TryFrom<PathBuf> for FileRef {
    type Error = ();

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let metadata = std::fs::metadata(&path).map_err(drop)?;
        Ok(FileRef {
            name: path
                .as_path()
                .file_name()
                .ok_or(())?
                .to_string_lossy()
                .into_owned(),
            len: metadata.len(),
            path,
        })
    }
}
