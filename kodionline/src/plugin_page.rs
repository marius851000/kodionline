use crate::{error_page::generate_error_page, get_absolute_plugin_path, Presentation};
use kodi_rust::{
    data::KodiResult, format_to_string, get_art_link_subcontent, get_media_link_resolved_url,
    get_media_link_subcontent, get_sub_content_from_parent, input::decode_input,
    input::encode_input, Kodi, PathAccessData, Setting, UserConfig,
};

use fluent_templates::Loader;
use log::error;
use maud::{html, Markup, PreEscaped};
use rocket::{http::RawStr, State};
use std::collections::HashMap;

use crate::{get_ui_locale, LOCALES};

#[allow(clippy::too_many_arguments)]
#[get("/plugin?<path>&<parent_path>&<input>&<parent_input>&<additional_input>&<c>")]
pub fn render_plugin(
    kodi: State<Kodi>,
    setting: State<Setting>,
    path: String,
    parent_path: Option<String>,
    input: Option<&RawStr>,
    parent_input: Option<&RawStr>,
    additional_input: Option<String>,
    c: Option<String>, //TODO: user_config_encoded in cookie
) -> Markup {
    let locale = get_ui_locale();

    let user_config_encoded = c;

    let user_config_in_url = UserConfig::new_from_optional_uri(user_config_encoded);
    let user_config = user_config_in_url;

    let final_config = setting
        .default_user_config
        .clone()
        .add_config_prioritary(user_config.clone());

    let mut input = input.map(|x| decode_input(&x)).unwrap_or_else(Vec::new);

    if let Some(value) = additional_input {
        input.push(value)
    }

    let current_access = PathAccessData {
        path: path.clone(),
        input,
        config: final_config.clone(),
    };

    let current_access_without_static = {
        let mut v = current_access.clone();
        v.config = user_config;
        v
    };

    let mut splited = path.split('.');
    splited.next();
    let plugin_type = match splited.next() {
        Some(value) => value.to_string(),
        None => {
            return generate_error_page(
                html!((LOCALES.lookup(&locale, "error-cant-get-plugin-type"))),
                &locale,
            )
            .kodi_url(Some(current_access_without_static.path))
            .build(&locale)
        }
    };

    let parent_access = PathAccessData::try_create_from_url(
        parent_path,
        parent_input.map(|x| x.as_str()),
        final_config,
    );

    let subcontent_from_parent = if let Some(ref parent_access_internal) = parent_access {
        get_sub_content_from_parent(&kodi, &parent_access_internal, &current_access.path)
    } else {
        None
    };

    match kodi.invoke_sandbox(&current_access) {
        Ok(KodiResult::Content(mut page)) => {
            match page.resolved_listitem {
                // contain a media
                Some(mut resolved_listitem) => {
                    if let Some(subcontent_from_parent) = subcontent_from_parent {
                        resolved_listitem.extend(subcontent_from_parent.listitem);
                    }

                    let media_base_url = match &resolved_listitem.path {
                        Some(url) => url.clone(),
                        None => return generate_error_page(html!((LOCALES.lookup(&locale, "error-no-media-found-on-page"))), &locale).kodi_url(Some(current_access_without_static.path)).build(&locale),
                    };

                    let title = html!((PreEscaped(resolved_listitem.get_display_html())));

                    let rendered_comment = resolved_listitem
                        .info
                        .comment
                        .clone()
                        .map(|comment| format_to_string(&comment));

                    let media_url = get_media_link_resolved_url(
                        &media_base_url,
                        &current_access.path,
                        current_access.input.clone(),
                        &current_access_without_static,
                    );

                    let media_type = if let Some(t) = resolved_listitem.category {
                        t
                    } else {
                        plugin_type
                    };

                    Presentation::new(
                        title,
                        html!(
                            div class = "main_media" {
                                @if media_type == "video" {
                                    video controls="" {
                                        source src=(media_url) {}
                                        //TODO: subtitles
                                    }
                                } @else if media_type == "audio" {
                                    audio controls="" {
                                        source src=(media_url) {}
                                    }
                                } @else {
                                    p { "can't detect the media type. Please download the file to use it." }
                                }
                            }

                            div class = "alt_media" {
                                @if !resolved_listitem.x_avalaible_languages.is_empty() {
                                    p { strong { "avalaible languages" }}
                                    ul class="language altlink" {
                                        @for language in resolved_listitem.x_avalaible_languages {
                                            li {
                                                a href=(get_absolute_plugin_path(
                                                    &{
                                                        let mut parent = current_access_without_static.clone();
                                                        parent.config.language_order.value = vec![language.clone()];
                                                        parent
                                                    },
                                                    parent_access.as_ref()
                                                )) {
                                                    (language)
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div class = "metadata" {
                                @if let Some(title) = resolved_listitem.info.title {
                                    p { b { (LOCALES.lookup(&locale, "word-title")) } " : " (title) }
                                }
                                @if let Some(album) = resolved_listitem.info.album {
                                    p { b { (LOCALES.lookup(&locale, "word-album")) } " : " (album) }
                                }
                                @if let Some(artist) = resolved_listitem.info.artist {
                                    p { b { (LOCALES.lookup(&locale, "word-artist")) } " : " (artist) }
                                }
                                @if let Some(year) = resolved_listitem.info.year {
                                    p { b { (LOCALES.lookup(&locale, "word-year")) } " : " (year) }
                                }
                                @if let Some(plot) = resolved_listitem.info.plot {
                                    p { b { (LOCALES.lookup(&locale, "word-plot")) } " : " (plot) }
                                }
                                @if let Some(genre) = resolved_listitem.info.genre {
                                    p { b { (LOCALES.lookup(&locale, "word-genre")) } " : " (genre) }
                                }
                                @if let Some(audio_language) = resolved_listitem.stream_info.audio.language {
                                    p { b { (LOCALES.lookup(&locale, "word-audio-language")) } " : " (audio_language)}
                                }
                                @if let Some(comment) = rendered_comment {
                                    p { PreEscaped { (comment) }}
                                }
                            }

                            h2 { (LOCALES.lookup(&locale, "word-download")) " :" }

                            ul class="download" {
                                li {
                                    a href=(media_url) {
                                        @if media_type == "video" {
                                            (LOCALES.lookup(&locale, "word-video"))
                                        } @else if media_type == "audio" {
                                            (LOCALES.lookup(&locale, "word-audio"))
                                        } @else {
                                            (LOCALES.lookup(&locale, "word-media"))
                                        }
                                    }
                                }
                            }
                        )
                    ).kodi_url(Some(current_access_without_static.path)).build(&locale)
                }
                // contain a folder
                None => {
                    let title_rendered = match subcontent_from_parent {
                        Some(subcontent) => {
                            html!((PreEscaped(subcontent.listitem.get_display_html())))
                        }
                        None => match setting.get_label_for_path(&current_access.path) {
                            Some(value) => html!((value)),
                            None => html!((current_access_without_static.path)),
                        },
                    };

                    let mut contain_playable_element = false;
                    Presentation::new(
                        title_rendered,
                        html!(
                            ul class="list_media" {
                                @for (loop_nb, sub_content) in page.sub_content.drain(..).enumerate() {
                                    li class="media_in_list" {
                                        a href=(get_absolute_plugin_path(&PathAccessData::new(sub_content.url.clone(), None, UserConfig::new_empty()), Some(&current_access))) {
                                            div class="subelem_title" { (PreEscaped(sub_content.listitem.get_display_html())) }
                                            @if let Some(thumb_category) = sub_content.listitem.get_thumb_category() {
                                                img class="illustration" src=(get_art_link_subcontent(
                                                    &sub_content,
                                                    thumb_category,
                                                    &current_access_without_static
                                                )) {}
                                            }

                                            @if let Some(plot) = &sub_content.listitem.info.plot {
                                                p class="plot" { (plot) }
                                            }
                                        }

                                        @if sub_content.listitem.is_playable() && plugin_type == "audio" {
                                            ({contain_playable_element = true; ""})
                                            br {}
                                            audio class="audiopreview" audiopreview_nb=(loop_nb.to_string()) preload=(if loop_nb == 0 { "auto" } else { "none" }) controls="true" {
                                                source src = (get_media_link_subcontent(
                                                    &sub_content,
                                                    &current_access_without_static,
                                                )) {}
                                            }
                                        }
                                    }
                                }
                            }

                            @if contain_playable_element && plugin_type == "audio" {
                                button id = "play_all" { (LOCALES.lookup(&locale, "play-all-music-sync")) }
                                script type="text/javascript" src="/static/musicplayer.js" {}
                            }
                        )
                    ).kodi_url(Some(current_access_without_static.path)).build(&locale)
                }
            }
        }
        Ok(KodiResult::Keyboard(keyboard)) => Presentation::new(
            html!((LOCALES.lookup(&locale, "input-required"))),
            html!(
                p { (LOCALES.lookup(&locale, "plugin-asked-value"))}
                form method="get" action="/plugin" {
                    @if let Some(heading) = keyboard.heading {
                        label for="additional_input" { (heading) }
                    }
                    input type=(if keyboard.hidden { "password" } else { "text" }) id="additional_input" name="additional_input" {}
                    br {}
                    input type="hidden" name="path" value=(current_access.path) {}
                    input type="hidden" name="input" value=(encode_input(&current_access.input)) {}
                    @if let Some(parent) = parent_access {
                        input type="hidden" name="parent_path" value=(parent.path) {}
                        input type="hidden" name="input_parent" value=(encode_input(&parent.input)) {}
                    }
                }
                @if let Some(default) = keyboard.default {
                    p { b { (LOCALES.lookup(&locale, "default-input")) } " : " (default) }
                }
            )
        ).kodi_url(Some(current_access_without_static.path)).build(&locale),
        Err(err) => {
            let error_args = {
                let mut map: HashMap<String, _> = HashMap::new();
                map.insert("url".into(), current_access.path.into());
                map
            };

            error!(
                "{}: {:?}",
                LOCALES.lookup_with_args(&locale, "error-getting-url", &error_args), err
            );
            generate_error_page(html!((format!("{}", err))), &locale).kodi_url(Some(current_access_without_static.path)).build(&locale)
        }
    }
}
