mod app_state;

use app_state::{AppState, StorageState};
use reshare_models::FileInfo;
use yew::prelude::*;

// ** Messages **

#[derive(Debug)]
enum Msg {
    GetFiles,
    KeyPhraseUpdated(String),
}

// ** Models **

struct Model {
    link: ComponentLink<Self>,
    app_state: AppState,
    files: Vec<FileInfo>,
}

impl Model {
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

    fn render_storage(&self) -> Html {
        let contents = if self.files.is_empty() {
            html! {
                <div class="row section no-files">
                    <p>{ "No files are currently available" }</p>
                </div>
            }
        } else {
            let download_url_root = self.app_state.download_url_root();

            html! {
                <table class="highlight">
                    <thead>
                        <tr>
                            <th>{ "File name" }</th>
                            <th>{ "Upload date" }</th>
                            <th>{ "Size" }</th>
                            <th>{ "Download" } </th>
                        </tr>
                    </thead>
                    <tbody>
                        { for self.files.iter().map(|f| into_table_row(f, &download_url_root)) }
                    </tbody>
                </table>
            }
        };

        html! {
            <>
            <div class="row section">
                <div class="col s12">
                    <h4 class="center-align">{ &self.app_state.storage_state }</h4>
                </div>
            </div>

            <div class="row section">
                <div class="col s10 offset-s1">
                    { contents }
                </div>
            </div>
            </>
        }
    }

    fn render_upload_button(&self) -> Html {
        html! {
            <div class="fixed-action-btn">
                <button class="waves-effect waves-light btn-floating btn-large direction-top upload-button">
                    <i class="large material-icons">{ "file_upload" }</i>
                </button>
            </div>
        }
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            app_state: Default::default(),
            files: vec![FileInfo::dummy()],
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::GetFiles => {
                self.files = self.app_state.fetch_files().unwrap();
                true
            }
            Msg::KeyPhraseUpdated(key_phrase) => {
                if key_phrase.is_empty() {
                    self.app_state.storage_state = StorageState::Public;
                } else {
                    self.app_state.storage_state = StorageState::Private { key_phrase };
                }

                false
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <>
            <div class="container">
                <div class="row spacer"></div>

                { self.render_header() }

                <div class="row spacer"></div>
                <div class="divider"></div>

                { self.render_storage() }
            </div>

            { self.render_upload_button() }

            </>
        }
    }
}

fn into_table_row(file_info: &FileInfo, download_url_root: &str) -> Html {
    use indicatif::HumanBytes;
    let human_readable_size = HumanBytes(file_info.size);
    let human_readable_date = file_info
        .upload_date
        .format("%Y %b %d - %H:%M:%S")
        .to_string();

    let download_path = format!("{}{}", download_url_root, file_info.name);
    html! {
        <tr>
            <td>{ &file_info.name }</td>
            <td>{ human_readable_date }</td>
            <td>{ human_readable_size }</td>
            <td class="centered-cell">
                <a href={ download_path } >
                    <i class="waves-effect waves-green circle material-icons ">{ "file_download" } </i>
                </a>
            </td>
        </tr>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<Model>();
}
