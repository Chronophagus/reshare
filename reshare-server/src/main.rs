mod downloader;
mod file_storage;
mod multipart;
mod uploader;

use actix_multipart::Multipart;
use actix_web::{
    body::Body, dev::HttpResponseBuilder, error::ResponseError, get, http::header,
    middleware::Logger, post, web, App, Error, HttpResponse, HttpServer,
};
use file_storage::FileStorage;
use reshare_models::{FileInfo, FileUploadStatus};
use std::sync::Mutex;
use uploader::UploadForm;

type Storage = Mutex<FileStorage>;

#[get("/list")]
async fn list(storage: web::Data<Storage>) -> Result<HttpResponse, Error> {
    list_impl(storage, None).await
}

#[get("/private/{keyphrase}")]
async fn list_private(
    storage: web::Data<Storage>,
    web::Path(keyphrase): web::Path<String>,
) -> Result<HttpResponse, Error> {
    list_impl(storage, Some(keyphrase)).await
}

async fn list_impl(
    storage: web::Data<Storage>,
    keyphrase: Option<String>,
) -> Result<HttpResponse, Error> {
    let guard = storage.lock().unwrap();

    let files: Vec<_> = guard.list(&keyphrase)?.collect();
    Ok(HttpResponse::Ok().json(files))
}

#[post("/upload")]
async fn upload(form_data: Multipart, storage: web::Data<Storage>) -> Result<HttpResponse, Error> {
    let mut upload_form = UploadForm::try_from_multipart(form_data).await?;
    let mut statuses = Vec::new();

    let keyphrase = upload_form.keyphrase;

    while let Some(file) = upload_form.files.next_file().await? {
        let upload_status = uploader::save_file(file.filename, file.file_stream).await;
        let mut storage = storage.lock().unwrap();

        statuses.push(upload_status);

        match statuses.last_mut().unwrap() {
            Ok(status_file_info) => {
                // Ensure unique name
                let file_info = std::iter::once(status_file_info.clone())
                    .chain((1..).map(|num| FileInfo {
                        name: format!("{}({})", status_file_info.name, num),
                        ..status_file_info.clone()
                    }))
                    .skip_while(|file_info| storage.is_file_exists(file_info, &keyphrase))
                    .next()
                    .unwrap();

                log::info!(
                    "Uploaded file: \"{}\", upload size: {}",
                    file_info.name,
                    file_info.size
                );

                storage.add_file(file_info.clone(), keyphrase.clone());
                *status_file_info = file_info;
            }
            Err(err) => {
                return Err(HttpResponseBuilder::new(err.status_code())
                    .json(transform_statuses(statuses))
                    .into());
            }
        }
    }

    Ok(HttpResponse::Ok().json(transform_statuses(statuses)))
}

#[get("/download/{file_name}")]
async fn download(
    web::Path(file_name): web::Path<String>,
    storage: web::Data<Storage>,
) -> Result<HttpResponse, Error> {
    download_impl(file_name, None, storage).await
}

#[get("/private/{keyphrase}/{file_name}")]
async fn download_private(
    web::Path((keyphrase, file_name)): web::Path<(String, String)>,
    storage: web::Data<Storage>,
) -> Result<HttpResponse, Error> {
    download_impl(file_name, Some(keyphrase), storage).await
}

async fn download_impl(
    file_name: String,
    keyphrase: Option<String>,
    storage: web::Data<Storage>,
) -> Result<HttpResponse, Error> {
    let file_info = {
        let guard = storage.lock().unwrap();
        guard
            .get_file(file_name, &keyphrase)
            .cloned()
            .ok_or_else(|| HttpResponse::NotFound().finish())?
    };

    let content_dispostion = header::ContentDisposition {
        disposition: header::DispositionType::Attachment,
        parameters: vec![header::DispositionParam::Filename(file_info.name.clone())],
    };

    let file_stream = downloader::download_file_stream(&file_info).await?;
    let response_body: Body = file_stream.into();

    Ok(HttpResponse::Ok()
        .header(header::CONTENT_DISPOSITION, content_dispostion)
        .body(response_body))
}

#[get("/upload")]
fn dummy_uploader(_storage: web::Data<Storage>) -> HttpResponse {
    let html = r#"<html>
        <head><title>Upload Test</title></head>
        <body>
            <form target="/" method="post" enctype="multipart/form-data">
                <input type="text" name="keyphrase"/>
                <input type="file" multiple name="file"/>
                <button type="submit">Submit</button>
            </form>
        </body>
    </html>"#;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "reshare_server=debug,actix_web=info");
    env_logger::init();

    let listen_addr = std::env::vars()
        .find(|(key, _)| key == "PORT")
        .map(|(_, val)| format!("0.0.0.0:{}", val))
        .unwrap_or_else(|| "0.0.0.0:8080".to_owned());

    let file_storage = web::Data::new(Mutex::new(FileStorage::new()));

    let app = move || {
        App::new()
            .app_data(file_storage.clone())
            .wrap(Logger::new("%a '%U' -> %s in %Ts"))
            .service(
                web::scope("/api")
                    .service(list)
                    .service(list_private)
                    .service(download)
                    .service(download_private)
                    .service(upload)
                    .service(dummy_uploader),
            )
    };

    HttpServer::new(app).bind(listen_addr)?.run().await?;
    uploader::cleanup().await;

    Ok(())
}

fn transform_statuses(results: Vec<uploader::Result<FileInfo>>) -> Vec<FileUploadStatus> {
    results.into_iter().map(|res| res.into()).collect()
}
