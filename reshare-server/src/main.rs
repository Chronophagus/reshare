mod multipart;
mod uploader;

use actix_multipart::Multipart;
use actix_web::{
    dev::HttpResponseBuilder, error::ResponseError, get, middleware::Logger, post, web, App, Error,
    HttpResponse, HttpServer, Responder,
};

use futures::StreamExt;
use reshare_models::{FileInfo, FileUploadStatus};
use uploader::UploadForm;

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().json(vec![FileInfo::dummy()])
}

#[post("/dummy")]
async fn upload_file(form_data: Multipart) -> Result<HttpResponse, Error> {
    let mut upload_form = UploadForm::try_from_multipart(form_data).await?;
    let mut statuses = Vec::new();

    log::debug!("Got keyphrase: {:?}", upload_form.keyphrase);

    while let Some(file) = upload_form.files.next_file().await? {
        log::debug!("Got filename {}", file.filename);

        let upload_status = uploader::save_file(file.filename, file.file_stream).await;
        statuses.push(upload_status);

        match statuses.last().unwrap() {
            Ok(_file_info) => (),
            Err(err) => {
                return Err(HttpResponseBuilder::new(err.status_code())
                    .json(transform_statuses(statuses))
                    .into())
            }
        }
    }

    Ok(HttpResponse::Ok().json(transform_statuses(statuses)))
}

#[get("/dummy")]
fn dummy_uploader() -> HttpResponse {
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

    let app = || {
        App::new()
            .wrap(Logger::default())
            .service(index)
            .service(upload_file)
            .service(dummy_uploader)
    };
    HttpServer::new(app).bind("0.0.0.0:8080")?.run().await
}

fn transform_statuses(results: Vec<uploader::Result<FileInfo>>) -> Vec<FileUploadStatus> {
    results.into_iter().map(|res| res.into()).collect()
}
