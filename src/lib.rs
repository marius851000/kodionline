#![allow(clippy::module_name_repetitions)]
mod kodi;
pub use kodi::{Kodi, KodiError};

pub mod data;

mod format;
pub use format::format_to_string;


// local use
use percent_encoding::{utf8_percent_encode, CONTROLS};

pub fn is_local_path(path: &str) -> bool {
    matches!(path.chars().next(), Some('/'))
}

pub fn encode_url(url: &str) -> String {
    utf8_percent_encode(url, CONTROLS).to_string()
}

#[test]
fn test_is_local_path() {
    assert!(is_local_path("/test.webm"));
    assert!(!is_local_path("http://example.org/media.webm"));
}

#[test]
fn test_encode_url() {
    assert_eq!(encode_url("http://î"), "http://%C3%AE");
}
