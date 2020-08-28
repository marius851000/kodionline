#![allow(clippy::module_name_repetitions)]
mod kodi;
pub use kodi::{Kodi, KodiError};

pub mod data;

mod format;
pub use format::format_to_string;


pub fn is_local_path(path: &str) -> bool {
    matches!(path.chars().next(), Some('/'))
}

#[test]
fn test_is_local_path() {
    assert!(is_local_path("/test.webm"));
    assert!(!is_local_path("http://example.org/media.webm"));
}
