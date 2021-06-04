use crate::utils::NeqAssign;
use yew::prelude::*;

#[derive(Properties, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploaderProps {
    #[prop_or_default]
    pub is_shown: bool,
}

pub struct Uploader {
    props: UploaderProps,
}

impl Component for Uploader {
    type Message = ();
    type Properties = UploaderProps;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        Self { props }
    }

    fn update(&mut self, _msg: Self::Message) -> ShouldRender {
        false
    }

    fn change(&mut self, new_props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(new_props)
    }

    fn view(&self) -> Html {
        if self.props.is_shown {
            html! {
            <>
              <div class="modal">
                <div class="modal-content">
                  <h4>{ "Upload files" }</h4>
                  <p>{ "TODO: Form" }</p>
                </div>
                <div class="modal-footer">
                  <button class="modal-close modal-action waves-effect btn-flat">{ "Close" }</button>
                </div>
              </div>
              <div class="modal-overlay"></div>
            </>
            }
        } else {
            html! {}
        }
    }
}
