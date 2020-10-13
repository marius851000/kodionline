use maud::{Markup, html, DOCTYPE};

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
            kodi_url: None
        }
    }

    pub fn kodi_url(mut self, url: Option<String>) -> Presentation {
        self.kodi_url = url;
        self
    }

    pub fn build(self) -> Markup {
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
                            a href="/" {"main page"}
                        }
                    }
                    h1 { (self.title) }
                }
                div id="content" { (self.content) }
                div id="footer" {
                    @if let Some(kodi_url) = self.kodi_url {
                        p {"kodi plugin url : " (kodi_url)}
                    }
                    p {
                        "website programmed by marius851000. Some data displayed on this site are not mine, namely nearly all data provided by the kodi's plugins."
                    }
                }
            }
        )
    }
}
