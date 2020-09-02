use crate::{
    escape_tag,
    input::{decode_input, encode_input},
    UserConfig,
};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct PathAccessData {
    pub path: String,
    pub input: Vec<String>,
    pub config: UserConfig,
}

impl PathAccessData {
    pub fn new(path: String, input: Option<&str>, config: UserConfig) -> Self {
        PathAccessData {
            path,
            input: decode_input(input),
            config,
        }
    }

    pub fn try_create_from_url(
        path: Option<String>,
        input: Option<&str>,
        config: UserConfig,
    ) -> Option<Self> {
        match path {
            Some(path_solved) => Some(Self::new(path_solved, input, config)),
            None => None,
        }
    }
}

#[derive(Serialize)]
pub struct PathAccessFormat {
    pub path_safe: String,
    pub path_escaped: String,
    pub input_encoded: String,
    pub config: UserConfig,
    pub config_uri_safe: String,
}

impl PathAccessFormat {
    pub fn new_from_pathaccessdata(path_access_data: PathAccessData) -> Self {
        PathAccessFormat {
            path_safe: utf8_percent_encode(&path_access_data.path, NON_ALPHANUMERIC).to_string(),
            path_escaped: escape_tag(path_access_data.path),
            input_encoded: encode_input(&path_access_data.input),
            config_uri_safe: path_access_data.config.encode_to_uri(),
            config: path_access_data.config,
        }
    }
}
