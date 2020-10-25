use crate::{get_absolute_plugin_path, Presentation};
use fluent_templates::Loader;
use kodi_rust::{PathAccessData, Setting, UserConfig};
use maud::{html, PreEscaped};
use rocket::State;

use crate::{get_ui_locale, LOCALES};

#[get("/")]
pub fn render_index(setting: State<Setting>) -> PreEscaped<String> {
    let locale = get_ui_locale();

    Presentation::new(
        html!((LOCALES.lookup(&locale, "kodionline"))),
        html!(
            p class="website_description" {
                (LOCALES.lookup(&locale, "presentation-head"))
                br {} (LOCALES.lookup(&locale, "presentation-advantage"))
                ul {
                    li {
                        (LOCALES.lookup(&locale, "presentation-advantage-1"))
                    }
                    li {
                        (LOCALES.lookup(&locale, "presentation-advantage-2"))
                    }
                    li {
                        (LOCALES.lookup(&locale, "presentation-advantage-3"))
                    }
                }
            }

            @if !setting.plugins_to_show.is_empty() {
                h2 { "avalaible plugins "}
                ul {
                    @for plugin in &setting.plugins_to_show {
                        li {
                            a href = (get_absolute_plugin_path(&PathAccessData::new(plugin.1.clone(), None, UserConfig::new_empty()), None)) { (plugin.0) }
                        }
                    }
                }
            }

            form method="get" action="/plugin" {
                label for="path_input" { "use a direct kodi plugin page" }
                input type="text" id="path_input" name="path" {}
                br {}
                input type="submit" value="go to the path" {}
            }
        )
    ).build(&locale)
}
