use fluent_templates::Loader;
use maud::{html, Markup, DOCTYPE};
use unic_langid::LanguageIdentifier;

use crate::LOCALES;

pub struct Presentation {
    pub title: Markup,
    pub content: Markup,
    pub kodi_url: Option<String>,
}

impl Presentation {
    pub fn new(title: Markup, content: Markup) -> Presentation {
        Presentation {
            title,
            content,
            kodi_url: None,
        }
    }

    pub fn kodi_url(mut self, url: Option<String>) -> Presentation {
        self.kodi_url = url;
        self
    }

    pub fn build(self, locale: &LanguageIdentifier) -> Markup {
        html!(
            (DOCTYPE)
            head {
                meta charset = "utf-8" {}
                title { (&self.title) }
                link rel="stylesheet" href="/static/kodionline.css" {}
            }
            body {
                div id="header" {
                    ul class="horizontallist" {
                        li {
                            a href="/" {(LOCALES.lookup(locale, "main-page"))}
                        }
                    }
                    h1 { (self.title) }
                }
                div id="content" { (self.content) }
                div id="footer" {
                    @if let Some(kodi_url) = self.kodi_url {
                        p {(LOCALES.lookup(locale, "kodi-plugin-url")) " : " (kodi_url)}
                    }
                    p {
                        (LOCALES.lookup(locale, "footer-legal"))
                    }
                }
            }
        )
    }
}
