use rocket_contrib::templates::Template;
use serde::Serialize;

#[derive(Serialize)]
pub struct PageError {
    errormessage: String,
}

pub fn generate_error_page(error_message: String) -> Template {
    let data = PageError {
        errormessage: error_message,
    };
    Template::render("error", data)
}
