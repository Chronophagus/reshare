use super::*;
use anyhow::{anyhow, bail};
use bytes::BytesMut;
use futures::{future, Stream};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use pin_project::pin_project;
use reqwest::{
    multipart::{Form, Part},
    Body,
};
use reshare_models::FileUploadStatus;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    ops::Deref,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tokio::{fs::File, io::AsyncRead, runtime as rt, sync::mpsc};
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

    let (reporter, mut monitor) = mpsc::channel(4096);

    let upload_status = ProgressStatus::new();
    let mut progress_bars = HashMap::new();

    for f in &files {
        let pb = upload_status.add(&f.name, f.len);
        progress_bars.insert(f.name.clone(), pb);
    }

    let upload_progress_task = std::thread::spawn(move || {
        upload_status.wait();
    });

    let upload_tasks: Vec<_> = files
        .into_iter()
        .map(|f| file_upload_task(query_url.clone(), f, key_phrase.clone(), reporter.clone()))
        .collect();

    drop(reporter);

    let rt = rt::Runtime::new()?;
    rt.block_on(async move {
        let monitor_task = tokio::spawn(async move {
            while let Some(progress_update) = monitor.recv().await {
                if let Some(progress_bar) = progress_bars.get(&progress_update.file_name) {
                    progress_bar.inc(progress_update.bytes_uploaded);
                }
            }

            for (_, bar) in progress_bars {
                bar.abandon();
            }
        });

        future::join_all(upload_tasks).await
    });

    upload_progress_task.join();

    Ok(())
}

async fn file_upload_task(
    url: Url,
    file_ref: FileRef,
    keyphrase: Option<String>,
    progress_reporter: mpsc::Sender<ProgressUpdate>,
) -> Result<FileUploadStatus> {
    let file = File::open(file_ref.path).await?;

    let mut file_stream: MonitoredStream<_> = FramedRead::new(file, BytesCodec::new()).into();
    let mut monitor = file_stream.take_monitor();

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

#[derive(Debug)]
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

#[pin_project]
struct MonitoredStream<R> {
    #[pin]
    stream: FramedRead<R, BytesCodec>,
    reporter: mpsc::UnboundedSender<u64>,
    monitor: Option<mpsc::UnboundedReceiver<u64>>,
}

impl<R> MonitoredStream<R> {
    fn take_monitor(&mut self) -> mpsc::UnboundedReceiver<u64> {
        self.monitor
            .take()
            .expect("take_monitor() can be called only once")
    }
}

impl<R: AsyncRead> From<FramedRead<R, BytesCodec>> for MonitoredStream<R> {
    fn from(stream: FramedRead<R, BytesCodec>) -> Self {
        let (reporter, monitor) = mpsc::unbounded_channel();

        Self {
            stream,
            reporter,
            monitor: Some(monitor),
        }
    }
}

impl<R: AsyncRead> Stream for MonitoredStream<R> {
    type Item = <FramedRead<R, BytesCodec> as Stream>::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.stream.poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                if let Err(e) = this.reporter.send(bytes.len() as u64) {
                    println!("Reporter error: {}", e);
                }

                Poll::Ready(Some(Ok(bytes)))
            }
            poll => poll,
        }
    }
}

struct ProgressUpdate {
    file_name: String,
    bytes_uploaded: u64,
}

#[derive(Clone)]
struct ProgressStatus {
    multiprogress: Arc<MultiProgress>,
}

impl ProgressStatus {
    fn new() -> Self {
        Self {
            multiprogress: Arc::new(MultiProgress::new()),
        }
    }

    fn wait(&self) {
        let _ = self.multiprogress.join();
    }

    fn add(&self, file_name: &str, file_size: u64) -> ProgressBar {
        let progress_bar = self.multiprogress.add(ProgressBar::new(file_size));

        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] {prefix} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} | {bytes_per_sec} (finishes in {eta})",
                )
                .progress_chars("=>-"),
        );

        progress_bar.set_prefix(file_name);
        progress_bar
    }
}
