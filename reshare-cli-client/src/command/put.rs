use super::*;
use crate::utils::{MonitoredStream, ProgressTracker};
use anyhow::{anyhow, bail};
use futures::future;
use reqwest::{
    multipart::{Form, Part},
    Body,
};
use reshare_models::FileUploadStatus;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    path::PathBuf,
};
use tokio::{fs::File, runtime as rt, sync::mpsc};
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

    let query_url = server_url.join("upload").unwrap();
    let key_phrase = args.key_phrase;

    let rt = rt::Runtime::new()?;

    let upload_tracker = ProgressTracker::new();

    let progress_bars = files.iter().fold(HashMap::new(), |mut map, file_ref| {
        let pb = upload_tracker.add(&file_ref.name, file_ref.len);
        map.insert(file_ref.name.clone(), pb);
        map
    });

    let upload_tracking_task = rt.spawn_blocking(move || {
        upload_tracker.show();
    });

    let (reporter, mut monitor) = mpsc::channel(4096);
    let upload_tasks: Vec<_> = files
        .iter()
        .map(|file_ref| {
            file_upload_task(
                query_url.clone(),
                file_ref.clone(),
                key_phrase.clone(),
                reporter.clone(),
            )
        })
        .collect();

    drop(reporter);

    let results = rt.block_on(async move {
        tokio::spawn(async move {
            while let Some(progress_update) = monitor.recv().await {
                if let Some(progress_bar) = progress_bars.get(&progress_update.file_name) {
                    progress_bar.inc(progress_update.bytes_uploaded);
                }
            }

            for (_, bar) in progress_bars {
                bar.abandon();
            }
        });

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
    progress_reporter: mpsc::Sender<ProgressUpdate>,
) -> Result<FileUploadStatus> {
    let file = File::open(file_ref.path).await?;

    let file_stream = FramedRead::new(file, BytesCodec::new());
    let (file_stream, mut monitor) = MonitoredStream::new(file_stream);

    let file_name = file_ref.name.clone();

    tokio::spawn(async move {
        while let Some(read_len) = monitor.recv().await {
            let _ = progress_reporter
                .send(ProgressUpdate {
                    file_name: file_name.clone(),
                    bytes_uploaded: read_len,
                })
                .await;
        }
    });

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

struct ProgressUpdate {
    file_name: String,
    bytes_uploaded: u64,
}
