mod files_view;
mod storage_state;
mod uploader;
mod utils;

use files_view::{FetchedFiles, FilesView, FilesViewMode};
use reshare_models::FileInfo;
use std::{cell::RefCell, rc::Rc};
use storage_state::StorageState;
use uploader::Uploader;
use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response, StatusCode};

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<ReshareModel>();
}

// ** Messages **

#[derive(Debug)]
enum Msg {
    GetFiles,
    KeyPhraseUpdated(String),
    ReceivedFiles(FetchedFiles),
    UploadButtonPressed,
}

// ** Models **

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewState {
    Downloader,
    Uploader,
}

impl ViewState {
    fn switch_state(&mut self) {
        let new_state = match self {
            Self::Downloader => Self::Uploader,
            Self::Uploader => Self::Downloader,
        };

        *self = new_state
    }
}

struct ReshareModel {
    link: ComponentLink<Self>,
    storage_state: StorageState,
    fetch_task: Option<FetchTask>,
    // RefCell is used here for optimization purposes
    files_view_mode: RefCell<Option<FilesViewMode>>,
    main_view_state: ViewState,
}

impl Component for ReshareModel {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let model = Self {
            link,
            storage_state: StorageState::Public,
            fetch_task: None,
            files_view_mode: RefCell::new(None),
            main_view_state: ViewState::Downloader,
        };

        model.link.send_message(Msg::GetFiles);
        model
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::GetFiles => {
                if self.fetch_task.is_some() {
                    return false;
                }

                let req = match Request::get(self.storage_state.fetch_files_url()).body(Nothing) {
                    Ok(req) => req,
                    Err(e) => {
                        log::error!("{}", e);
                        return false;
                    }
                };

                let storage_state = self.storage_state.clone();

                let callback = self.link.callback(
                    move |response: Response<Json<Result<Vec<FileInfo>, anyhow::Error>>>| {
                        let Json(data) = if response.status() == StatusCode::NOT_FOUND {
                            log::error!("Storage not found");
                            Json(Ok(Vec::new()))
                        } else {
                            response.into_body()
                        };

                        let file_list = data.unwrap_or_else(|e| {
                            log::error!("{}", e);
                            Vec::new()
                        });

                        Msg::ReceivedFiles(FetchedFiles {
                            file_list,
                            storage_state: storage_state.clone(),
                        })
                    },
                );

                let fetch_task = FetchService::fetch(req, callback).expect("Fetching must work");
                self.fetch_task = Some(fetch_task);
                *self.files_view_mode.borrow_mut() = Some(FilesViewMode::ShowProgress);

                true
            }
            Msg::ReceivedFiles(fetched_files) => {
                *self.files_view_mode.borrow_mut() = Some(FilesViewMode::ShowFiles(fetched_files));
                self.fetch_task = None;
                true
            }
            Msg::KeyPhraseUpdated(key_phrase) => {
                if key_phrase.is_empty() {
                    self.storage_state = StorageState::Public;
                } else {
                    self.storage_state = StorageState::Private { key_phrase };
                }

                false
            }
            Msg::UploadButtonPressed => {
                self.main_view_state.switch_state();

                if let ViewState::Downloader = self.main_view_state {
                    self.link.send_message(Msg::GetFiles);
                }

                true
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let files_view_mode = Rc::new(self.files_view_mode.borrow_mut().take().unwrap_or_default());

        html! {
            <>
            <div class="container">
                <div class="row spacer"></div>

                { self.render_header() }

                <div class="row spacer"></div>
                <div class="divider"></div>

                <FilesView mode=files_view_mode />
            </div>

            { self.render_upload_button() }

            <Uploader is_shown=self.main_view_state == ViewState::Uploader />

            </>
        }
    }
}

impl ReshareModel {
    fn render_header(&self) -> Html {
        let key_phrase_update_cb = self
            .link
            .callback(|e: InputData| Msg::KeyPhraseUpdated(e.value));

        let get_files_cb = self.link.callback(|_| Msg::GetFiles);

        let enter_pressed_cb = self.link.batch_callback(|e: KeyboardEvent| {
            if e.key() == "Enter" {
                vec![Msg::GetFiles]
            } else {
                Vec::new()
            }
        });

        html! {
            <div class="row">
                <div class="col s12">
                    <div class="input-field col s12">
                        <input
                           id="key_phrase"
                           type="text"
                           class="validate"
                           oninput=key_phrase_update_cb
                           onkeypress=enter_pressed_cb />
                        <label for="key_phrase">{ "Enter a key phrase" }</label>
                    </div>
                    <div class="col s12">
                        <button
                           class="waves-effect waves-light btn-large get-files-btn"
                           name="Keyphrase",
                           onclick=get_files_cb>
                               { "Get files" }
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_upload_button(&self) -> Html {
        html! {
            <div class="fixed-action-btn">
                <button
                  class="waves-effect waves-light btn-floating btn-large direction-top upload-button"
                  onclick=self.link.callback(|_| Msg::UploadButtonPressed)>
                    <i class="large material-icons">{ "file_upload" }</i>
                </button>
            </div>
        }
    }
}
