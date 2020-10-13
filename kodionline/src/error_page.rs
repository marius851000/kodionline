use crate::Presentation;
use maud::{html, Markup};
use serde::Serialize;

#[derive(Serialize)]
pub struct PageError {
    errormessage: String,
}

pub fn generate_error_page(error_message: Markup) -> Presentation {
    Presentation::new(
        html!("kodionline: error"),
        html!(
            div class="errormessage" {
                p { "the following error happened :"}
                p { (error_message) }
            }
        )
    )
}
