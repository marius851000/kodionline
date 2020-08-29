use crate::input::decode_input;
use rocket::http::RawStr;

pub struct PathAccessData {
    pub path: String,
    pub input: Vec<String>,
}

impl PathAccessData {
    pub fn try_create_from_url(path: Option<String>, input: Option<&RawStr>) -> Option<Self> {
        match path {
            Some(path_solved) => Some(PathAccessData {
                path: path_solved,
                input: decode_input(input),
            }),
            None => None,
        }
    }
}
