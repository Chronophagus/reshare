#![recursion_limit = "1024"]

use yew::prelude::*;

struct Model {
    link: ComponentLink<Self>,
}

impl Component for Model {
    type Message = ();
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self { link }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        false
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <>
            <div class="container">
                <div class="row spacer"></div>

                <div class="row">
                    <div class="col s12">
                        <div class="input-field col s12">
                            <input id="key_phrase" type="text" class="validate" />
                            <label for="key_phrase">{ "Enter a key phrase" }</label>
                        </div>
                        <div class="col s12">
                            <button class="waves-effect waves-light btn-large get-files-btn" name="Keyphrase">
                                { "Get files" }
                            </button>
                        </div>
                    </div>
                </div>

                <div class="row spacer"></div>

                <div class="divider"></div>

                <div class="row section">
                    <div class="col s12">
                        <h4 class="center-align">{ "Contents of the public storage" }</h4>
                    </div>
                </div>

                <div class="row section">
                    <div class="col s10 offset-s1">
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
                                <tr>
                                    <td>{ "Reshare server" }</td>
                                    <td>{ "Today" }</td>
                                    <td>{ "100.5 KB" }</td>
                                    <td class="centered-cell"><a href="#" ><i class="waves-effect waves-green circle material-icons ">{ "file_download" }</i></a></td>
                                </tr>
                                <tr>
                                    <td>{ "Reshare cli client" }</td>
                                    <td>{ "2020-12-01 21:32" }</td>
                                    <td>{ "99 MB" }</td>
                                    <td class="centered-cell"><a href="#"><i class="waves-effect waves-green circle material-icons ">{ "file_download" }</i></a></td>
                                </tr>
                                <tr>
                                    <td>{ "reshare-web-client" }</td>
                                    <td>{ "2020-12-01 21:33" }</td>
                                    <td>{ "100 MB" }</td>
                                    <td class="centered-cell"><a href="#" ><i class="waves-effect waves-green circle material-icons ">{ "file_download" }</i></a></td>
                                </tr>
                                <tr>
                                    <td>{ "Random file" }</td>
                                    <td>{ "2020-12-01 21:34" }</td>
                                    <td>{ "1 GB" }</td>
                                    <td class="centered-cell"><a href="#" ><i class="waves-effect waves-green material-icons circle">{ "file_download" }</i></a></td>
                                </tr>
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>

            <div class="fixed-action-btn">
                <button class="waves-effect waves-light btn-floating btn-large direction-top upload-button">
                    <i class="large material-icons">{ "file_upload" }</i>
                </button>
            </div>
            </>
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}
