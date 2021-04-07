use super::*;
use anyhow::bail;
use futures::{future, TryStreamExt};
use reqwest::multipart::{Form, Part};
use reqwest::Body;
use reshare_models::FileUploadStatus;
use tokio::fs::File;
use tokio::runtime as rt;
use tokio_util::codec::{BytesCodec, FramedRead};

use std::convert::{TryFrom, TryInto};
use std::path::PathBuf;

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

    let upload_tasks = files
        .into_iter()
        .map(|f| file_upload_task(query_url.clone(), f, key_phrase.clone()));

    let rt = rt::Runtime::new()?;
    dbg!(rt.block_on(future::join_all(upload_tasks)));

    Ok(())
}

async fn file_upload_task(
    url: Url,
    file_ref: FileRef,
    keyphrase: Option<String>,
) -> Result<FileUploadStatus> {
    let file = File::open(file_ref.path).await?;
    let file_stream = FramedRead::new(file, BytesCodec::new());

    let file_part = Part::stream(Body::wrap_stream(file_stream)).file_name(file_ref.name.clone());

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
    let response = response.into_iter().next().unwrap();

    if let FileUploadStatus::Success(_) = &response {
        println!("{} - OK", file_ref.name);
    } else {
        println!("{} - FAIL", file_ref.name);
    }

    Ok(response)
}

#[derive(Debug)]
struct FileRef {
    name: String,
    path: PathBuf,
}

impl TryFrom<PathBuf> for FileRef {
    type Error = ();

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Ok(FileRef {
            name: path
                .as_path()
                .file_name()
                .ok_or(())?
                .to_string_lossy()
                .into_owned(),
            path,
        })
    }
}
