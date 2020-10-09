use rocket::State;
use kodi_rust::Setting;
use crate::{get_absolute_plugin_path, format_standard_page};
use maud::{html, PreEscaped};

#[get("/")]
pub fn render_index(setting: State<Setting>) -> PreEscaped<String> {
    format_standard_page("kodi online", html!(
        p class="website_description" {
            "kodi online is a service that allow to browse the content of kodi addon in a web browser."
            " Metadata will be fetched by the server, but the media will be viewed/downloaded by your computer from the original site."
            br {} "This may have some advantage :"
            ul {
                li {
                    "it permit greater anonymity: individual user tracking are almost always done when viewing the webpage that contain the video, not the video itself"
                }
                li {
                    "it allow to have a unified interface (while the original interface should be almost always better than this one, this one have a download button)"
                }
                li {
                    "it have the same advantage to browsing the plugin in kodi (having a different presentation than the original, for the worst and the better)"
                }
            }
        }

        @if !setting.plugins_to_show.is_empty() {
            h2 { "avalaible plugins "}
            ul {
                @for plugin in &setting.plugins_to_show {
                    li {
                        a href = (get_absolute_plugin_path(&plugin.1)) { (plugin.0) }
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
    ), None)
}
