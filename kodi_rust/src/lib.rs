#![allow(clippy::module_name_repetitions)]

mod kodi;
pub use kodi::{Kodi, KodiCallError, KodiError};

pub mod data;

mod format;
pub use format::format_to_string;

pub mod input;

mod setting;
pub use setting::Setting;

mod pathaccessdata;
pub use pathaccessdata::{PathAccessData, PathAccessFormat};

mod getlink;
pub use getlink::*;

mod user_config;
pub use user_config::{OverridableVec, UserConfig};

// local use
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

pub fn should_serve_file(path: &str) -> bool {
    matches!(path.chars().next(), Some('/')) || path.starts_with("plugin://")
}

static URLENCODE: AsciiSet = CONTROLS.add(b' ');
static HTMLENCODE: AsciiSet = CONTROLS.add(b'\'').add(b'"').add(b'&').add(b'<').add(b'>');

pub fn escape_tag(value: String) -> String {
    value
        .replace("\"", "&quot;")
        .replace("\'", "&#39;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("&", "&amp;") //TODO: use the library
}

pub fn encode_url(url: &str) -> String {
    utf8_percent_encode(url, &URLENCODE).to_string()
}

pub fn extend_option<T: Clone>(source: &mut Option<T>, extender: Option<T>) {
    if extender.is_some() {
        *source = extender
    }
}

#[test]
fn test_extend_option() {
    let mut changed = None;
    extend_option(&mut changed, None);
    assert_eq!(changed, None);
    extend_option(&mut changed, Some(1));
    assert_eq!(changed, Some(1));
    changed = Some(2);
    extend_option(&mut changed, None);
    assert_eq!(changed, Some(2));
    extend_option(&mut changed, Some(5));
    assert_eq!(changed, Some(5));
}

#[test]
fn test_should_serve_file() {
    assert!(should_serve_file("/test.webm"));
    assert!(should_serve_file("plugin://stuff/url"));
    assert!(!should_serve_file("http://example.org/media.webm"));
}

#[test]
fn test_encode_url() {
    assert_eq!(encode_url("http://î h"), "http://%C3%AE%20h");
}

//TODO: find where to put this

use data::{KodiResult, SubContent};
use log::error;

pub fn get_sub_content_from_parent(
    kodi: &Kodi,
    parent_access: &PathAccessData,
    child_path: &str,
) -> Option<SubContent> {
    match kodi.invoke_sandbox(&parent_access) {
        Ok(KodiResult::Content(parent_page)) => {
            let mut result = None;
            for sub_content in parent_page.sub_content {
                if sub_content.url == child_path {
                    result = Some(sub_content);
                    break;
                };
            }
            result
        }
        Ok(result) => {
            error!(
                "an input was asked while asking for the parent path {} (result: {:?})",
                parent_access.path, result
            );
            None
        }
        Err(err) => {
            error!(
                "got {:?} while trying to get the parent path {}",
                err, parent_access.path,
            );
            None
        }
    }
}
