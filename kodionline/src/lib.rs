#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use maud::{PreEscaped, DOCTYPE, Markup, html};
use kodi_rust::encode_url;


pub mod plugin_page;

pub mod error_page;

pub mod redirect_page;

pub mod index_page;

pub fn get_absolute_plugin_path(text: &str) -> PreEscaped<String> {
    PreEscaped(format!("/plugin?path={}", encode_url(text)))
}

pub fn format_standard_page(page_title: &str, page_content: Markup, additional_footer: Option<Markup>) -> Markup {
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
