#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use kodi_rust::{input::encode_input, urlencode, PathAccessData};
use maud::{html, Markup, PreEscaped, DOCTYPE};

pub mod plugin_page;

pub mod error_page;

pub mod redirect_page;

pub mod index_page;

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

pub fn format_standard_page(
    page_title: Markup,
    page_content: Markup,
    additional_footer: Option<Markup>,
) -> Markup {
    html!(
        (DOCTYPE)
        head {
            meta charset="utf-8" {}
            title { (page_title) }
            link rel="stylesheet" href="/static/kodionline.css" {}
        }
        body {
            div id="header" {
                ul class="horizontallist" {
                    li {
                        a href="/" {"main page"}
                    }
                }
                h1 { (page_title) }
            }
            div id="content" { (page_content) }
            div id="footer" {
                @if let Some(footer) = additional_footer {
                    (footer)
                }
                p {
                    "website programmed by marius851000. Some data displayed on this site are not mine, namely nearly all data provided by the kodi's plugins"
                }
            }
        }
    )
}
