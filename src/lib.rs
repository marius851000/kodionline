#![feature(proc_macro_hygiene, decl_macro)]
#![allow(clippy::module_name_repetitions)]

#[macro_use]
extern crate rocket;

mod kodi;
pub use kodi::{Kodi, KodiError};

pub mod data;

mod format;
pub use format::format_to_string;

pub mod input;

mod setting;
pub use setting::Setting;

mod pathaccessdata;
pub use pathaccessdata::PathAccessData;

mod getlink;
pub use getlink::*;

pub mod plugin_page;

pub mod error_page;

pub mod redirect_page;

mod user_config;
pub use user_config::UserConfig;

// local use
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

pub fn should_serve_file(path: &str) -> bool {
    matches!(path.chars().next(), Some('/')) || path.starts_with("plugin://")
}

static URLENCODE: AsciiSet = CONTROLS.add(b' ');

pub fn encode_url(url: &str) -> String {
    utf8_percent_encode(url, &URLENCODE).to_string()
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
use rocket::State;

pub fn get_sub_content_from_parent(
    kodi: &State<Kodi>,
    parent_access: &PathAccessData,
    child_path: &str,
) -> Option<SubContent> {
    match kodi.invoke_sandbox(&parent_access.path, parent_access.input.clone()) {
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
