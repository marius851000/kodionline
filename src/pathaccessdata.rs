use crate::{input::decode_input, UserConfig};
use rocket::http::RawStr;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct PathAccessData {
    pub path: String,
    pub input: Vec<String>,
    pub config: UserConfig,
}

impl PathAccessData {
    pub fn new(path: String, input: Option<&RawStr>, config: UserConfig) -> Self {
        PathAccessData {
            path,
            input: decode_input(input),
            config,
        }
    }

    pub fn try_create_from_url(
        path: Option<String>,
        input: Option<&RawStr>,
        config: UserConfig,
    ) -> Option<Self> {
        match path {
            Some(path_solved) => Some(Self::new(path_solved, input, config)),
            None => None,
        }
    }
}
