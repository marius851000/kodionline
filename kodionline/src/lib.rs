#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use kodi_rust::{input::encode_input, urlencode, PathAccessData};
use maud::{html, Markup, PreEscaped, DOCTYPE};

pub mod plugin_page;

pub mod error_page;

pub mod redirect_page;

pub mod index_page;

mod presentation;
pub use presentation::Presentation;

pub fn get_absolute_plugin_path(
    main: &PathAccessData,
    parent: Option<&PathAccessData>,
) -> PreEscaped<String> {
    PreEscaped(format!(
        "/plugin?path={}{}{}{}",
        urlencode(&main.path),
        if !main.input.is_empty() {
            format!("&input={}", encode_input(&main.input))
        } else {
            String::new()
        },
        if !main.config.is_empty() {
            format!("&c={}", main.config.encode_to_uri())
        } else {
            String::new()
        },
        if let Some(parent) = parent {
            format!(
                "&parent_path={}{}",
                urlencode(&parent.path),
                if !parent.input.is_empty() {
                    format!("&parent_input={}", encode_input(&parent.input))
                } else {
                    String::new()
                },
            )
        } else {
            String::new()
        }
    ))
}
