mod file_storage;
mod multipart;
mod uploader;

use actix_multipart::Multipart;
use actix_web::{
    dev::HttpResponseBuilder, error::ResponseError, get, middleware::Logger, post, web, App, Error,
    HttpResponse, HttpServer, Responder,
};
use file_storage::FileStorage;
use reshare_models::{FileInfo, FileUploadStatus};
use std::sync::Mutex;
use uploader::UploadForm;

type Storage = Mutex<FileStorage>;

#[get("/")]
async fn index(storage: web::Data<Storage>) -> impl Responder {
    let guard = storage.lock().unwrap();

    let files: Vec<_> = guard.list(&None).unwrap().collect();
    HttpResponse::Ok().json(files)
}

#[post("/dummy")]
async fn upload_file(
    form_data: Multipart,
    storage: web::Data<Storage>,
) -> Result<HttpResponse, Error> {
    let mut upload_form = UploadForm::try_from_multipart(form_data).await?;
    let mut statuses = Vec::new();

    let keyphrase = &upload_form.keyphrase;

    while let Some(file) = upload_form.files.next_file().await? {
        let upload_status = uploader::save_file(file.filename, file.file_stream).await;
        let mut guard = storage.lock().unwrap();

        statuses.push(upload_status.and_then(|file_info| {
            if guard.is_file_exists(&file_info, &keyphrase) {
                Err(uploader::UploadError::FileExists(file_info))
            } else {
                Ok(file_info)
            }
        }));

        match statuses.last().unwrap() {
            Ok(file_info) => {
                guard.add_file(file_info.clone(), keyphrase.clone());
            }
            Err(err) => {
                if let uploader::UploadError::FileExists(file_info) = err {
                    uploader::schedule_removal(file_info.clone());
                }

                return Err(HttpResponseBuilder::new(err.status_code())
                    .json(transform_statuses(statuses))
                    .into());
            }
        }
    }

    Ok(HttpResponse::Ok().json(transform_statuses(statuses)))
}

#[get("/dummy")]
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

    let file_storage = web::Data::new(Mutex::new(FileStorage::new()));

    let app = move || {
        App::new()
            .app_data(file_storage.clone())
            .wrap(Logger::default())
            .service(index)
            .service(upload_file)
            .service(dummy_uploader)
    };

    HttpServer::new(app).bind("0.0.0.0:8080")?.run().await?;

    // TODO:
    // uploader::clear_upload_dir().await?

    Ok(())
}

fn transform_statuses(results: Vec<uploader::Result<FileInfo>>) -> Vec<FileUploadStatus> {
    results.into_iter().map(|res| res.into()).collect()
}
