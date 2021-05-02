use super::FetchedFiles;
use reshare_models::FileInfo;
use std::rc::Rc;
use yew::prelude::*;
use yew::Properties;

pub enum FilesViewMode {
    Idle,
    ShowProgress,
    ShowFiles(FetchedFiles),
}

impl std::cmp::PartialEq for FilesViewMode {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (Self::Idle, Self::Idle) => true,
            (Self::ShowProgress, Self::ShowProgress) => true,
            _ => false,
        }
    }
}

impl Default for FilesViewMode {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct Mode {
    #[prop_or_default]
    pub mode: Rc<FilesViewMode>,
}

pub struct FilesView {
    link: ComponentLink<Self>,
    view_mode: Mode,
}

impl Component for FilesView {
    type Message = ();
    type Properties = Mode;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            view_mode: props,
            link,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        false
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.view_mode == props {
            false
        } else {
            self.view_mode = props;
            true
        }
    }

    fn view(&self) -> Html {
        match *self.view_mode.mode {
            FilesViewMode::Idle => html! {},
            FilesViewMode::ShowProgress => html! {
                <div class="row section valign-wrapper no-files">
                    <div class="col s12 centered-block">
                        <div class="preloader-wrapper active">
                          <div class="spinner-layer green-only">
                            <div class="circle-clipper left">
                              <div class="circle"></div>
                            </div>
                            <div class="gap-patch">
                              <div class="circle"></div>
                            </div>
                            <div class="circle-clipper-right">
                              <div class="circle"></div>
                            </div>
                          </div>
                        </div>
                    </div>
                </div>
            },
            FilesViewMode::ShowFiles(ref fetched_files) => {
                let download_url_root = fetched_files.storage_state.download_url_root();

                let contents = if fetched_files.file_list.is_empty() {
                    html! {
                        <div class="row section no-files">
                            <div class="col s12">
                                <p class="center-align">{ "No files are currently available" }</p>
                            </div>
                        </div>
                    }
                } else {
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
                                { for fetched_files.file_list.iter().map(|f| into_table_row(f, &download_url_root)) }
                            </tbody>
                        </table>
                    }
                };

                html! {
                    <>
                    <div class="row section">
                        <div class="col s10 offset-s1">
                            <h4 class="center-align">{ &fetched_files.storage_state }</h4>
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
                    <i class="waves-effect waves-green circle material-icons ">
                        { "file_download" }
                    </i>
                </a>
            </td>
        </tr>
    }
}
