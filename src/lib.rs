#![allow(clippy::module_name_repetitions)]
mod kodi;
pub use kodi::{Kodi, KodiError};

pub mod data;

mod format;
pub use format::format_to_string;

pub fn is_local_path(path: &str) -> bool {
    matches!(path.chars().next(), Some('/'))
}

struct EncoderUrl;
impl pct_str::Encoder for EncoderUrl {
    fn encode(&self, c: char) -> bool {
        if c == ' ' {
            return true
        };
        !c.is_ascii()
    }
}

pub fn encode_url(url: &str) -> String {
    pct_str::PctString::encode(url.chars(), EncoderUrl).as_str().to_string()
}

#[test]
fn test_is_local_path() {
    assert!(is_local_path("/test.webm"));
    assert!(!is_local_path("http://example.org/media.webm"));
}
