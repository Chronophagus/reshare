use super::*;
use crate::utils::{
    progress_tracker::{ProgressReporter, ProgressTracker, ProgressUpdate},
    ChanConnector, MonitoredStream,
};
use anyhow::{anyhow, bail};
use bytes::Bytes;
use futures::{future, Stream, StreamExt};
use reqwest::{StatusCode, Url};
use tokio::{fs::File, io::AsyncWriteExt, runtime::Runtime};

pub fn execute(args: GetArgs) -> Result<()> {
    let server_url = load_configuration()?;

    let file_names = args.file_list;

    if file_names.is_empty() {
        bail!("No files to download");
    }

    let query_url = match args.key_phrase {
        Some(key_phrase) => server_url
            .join("api/")?
            .join("private/")?
            .join(&format!("{}/", key_phrase))?,
        None => server_url.join("api/")?.join("download/")?,
    };

    let rt = Runtime::new()?;

    let get_file_info_tasks = file_names
        .into_iter()
        .map(|file_name| get_file_info(query_url.clone(), file_name.clone()));

    let results = rt.block_on(async move {
        let (files, errors): (Vec<_>, Vec<_>) = future::join_all(get_file_info_tasks)
            .await
            .into_iter()
            .partition(Result::is_ok);

        for err in errors {
            println!("Err: {}", err.unwrap_err());
        }

        let files: Vec<_> = files.into_iter().map(Result::unwrap).collect();

        let mut download_tracker = ProgressTracker::new();

        for file_info in &files {
            download_tracker.add_bar(file_info.name.clone(), file_info.len);
        }

        let download_tasks: Vec<_> = files
            .into_iter()
            .map(|file_info| download_file(file_info, download_tracker.get_reporter()))
            .collect();

        let download_tracking_task = download_tracker.spawn();

        let results = future::join_all(download_tasks).await;
        let _ = download_tracking_task.await;

        results
    });

    for result in results {
        if let Err(e) = result {
            println!("Err: {}", e);
        }
    }

    Ok(())
}

async fn get_file_info(
    query_url: Url,
    file_name: String,
) -> Result<FileInfo<impl Stream<Item = ByteChunk>>> {
    let file_url = query_url.join(&file_name)?;

    let response = reqwest::get(file_url).await?;

    if !response.status().is_success() {
        if response.status() == StatusCode::NOT_FOUND {
            bail!("{} not found", file_name);
        } else {
            let contents = response.text().await?;
            bail!("{}", contents);
        }
    }

    let file_len = response
        .content_length()
        .ok_or_else(|| anyhow!("{} - unknown file size", file_name))?;

    Ok(FileInfo {
        name: file_name,
        stream: response.bytes_stream(),
        len: file_len,
    })
}

async fn download_file<S: Stream<Item = ByteChunk> + Unpin>(
    file_info: FileInfo<S>,
    reporter: ProgressReporter,
) -> Result<()> {
    let file_name = file_info.name.clone();

    let mut file = create_file(file_info.name).await?;
    let (mut stream, monitor) = MonitoredStream::new(file_info.stream);

    ChanConnector::connect_with(monitor, reporter, move |bytes_written| ProgressUpdate {
        file_name: file_name.clone(),
        bytes_transmitted: bytes_written,
    })
    .seal();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
    }

    Ok(())
}

async fn create_file(file_name: String) -> Result<File> {
    use std::io::ErrorKind;

    for file_name in
        std::iter::once(file_name.clone()).chain((1..).map(|num| format!("{}({})", file_name, num)))
    {
        match File::create(&file_name).await {
            Err(e) if e.kind() == ErrorKind::AlreadyExists || e.kind() == ErrorKind::Other => {
                continue
            }
            result => return result.map_err(|e| anyhow!("{} - {}", file_name, e)),
        }
    }

    unreachable!();
}

type ByteChunk = reqwest::Result<Bytes>;

struct FileInfo<S> {
    name: String,
    stream: S,
    len: u64,
}

impl<S> std::fmt::Debug for FileInfo<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileInfo")
            .field("name", &self.name)
            .field("len", &self.len)
            .finish()
    }
}
