use crate::format_standard_page;
use maud::{html, Markup};
use serde::Serialize;

#[derive(Serialize)]
pub struct PageError {
    errormessage: String,
}

pub fn generate_error_page(error_message: Markup) -> Markup {
    format_standard_page(
        html!("kodionline: error"),
        html!(
            div class="errormessage" {
                p { "the following error happened :"}
                p { (error_message) }
            }
        ),
        None,
    )
}
