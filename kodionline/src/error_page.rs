use crate::Presentation;
use fluent_templates::Loader;
use maud::{html, Markup};
use serde::Serialize;
use unic_langid::LanguageIdentifier;

use crate::LOCALES;

#[derive(Serialize)]
pub struct PageError {
    errormessage: String,
}

pub fn generate_error_page(error_message: Markup, locale: &LanguageIdentifier) -> Presentation {
    Presentation::new(
        html!((LOCALES.lookup(locale, "error-title"))),
        html!(
            div class="errormessage" {
                p { (LOCALES.lookup(locale, "intro-error")) }
                p { (error_message) }
            }
        ),
    )
}
