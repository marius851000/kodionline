use crate::{error_page::generate_error_page, format_standard_page, get_absolute_plugin_path};
use kodi_rust::{
    data::{KodiResult, ListItem, SubContent},
    format_to_string, get_art_link_subcontent, get_media_link_resolved_url,
    get_media_link_subcontent, get_sub_content_from_parent,
    input::decode_input,
    input::encode_input,
    Kodi, PathAccessData, PathAccessFormat, Setting, UserConfig,
};

use log::error;
use maud::{html, Markup, PreEscaped};
use rocket::{http::RawStr, State};
use serde::Serialize;

#[derive(Serialize)]
pub struct PagePluginMedia {
    item: ListItem,
    access: PathAccessFormat,
    parent: Option<PathAccessFormat>,
    plugin_type: String,
    title_rendered: Option<String>,
    media_url: String,
    rendered_comment: Option<String>,
}

#[derive(Serialize)]
pub struct SubContentDisplay {
    data: SubContent,
    label_html: String,
    is_playable: bool,
    media_url: String,
    art_url: Option<String>,
}

#[derive(Serialize)]
pub struct PagePluginFolder {
    all_sub_content: Vec<SubContentDisplay>,
    access: PathAccessFormat,
    parent: Option<PathAccessFormat>,
    plugin_type: String,
    title_rendered: Option<String>,
}

#[derive(Serialize)]
pub struct PagePluginKeyboard {
    plugin_type: String,
    access: PathAccessFormat,
    parent: Option<PathAccessFormat>,
    title_rendered: Option<String>,
    keyboard_default: Option<String>,
    keyboard_heading: Option<String>,
    keyboard_hidden: bool,
}

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
    let user_config_encoded = c;

    let user_config_in_url = UserConfig::new_from_optional_uri(user_config_encoded);
    let user_config = user_config_in_url;

    let final_config = setting
        .default_user_config
        .clone()
        .add_config_prioritary(user_config.clone());

    let mut splited = path.split('.');
    splited.next();
    let plugin_type = match splited.next() {
        Some(value) => value.to_string(),
        None => return generate_error_page(html!("impossible to get type of extension")),
    };

    let mut input = input
        .map(|x| decode_input(&x))
        .unwrap_or_else(|| Vec::new());

    if let Some(value) = additional_input {
        input.push(value)
    }

    let current_access = PathAccessData {
        path,
        input,
        config: final_config.clone(),
    };

    let current_access_without_static = {
        let mut v = current_access.clone();
        v.config = user_config;
        v
    };

    let parent_access = PathAccessData::try_create_from_url(
        parent_path.clone(),
        parent_input.map(|x| x.as_str()),
        final_config,
    );

    let subcontent_from_parent = if let Some(ref parent_access_internal) = parent_access {
        get_sub_content_from_parent(&kodi, &parent_access_internal, &current_access.path)
    } else {
        None
    };

    let footer = html!( p { "kodi plugin url : " (current_access_without_static.path)});

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
                        None => return generate_error_page(html!("no media found for this page")),
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

                    /*let data = PagePluginMedia {
                        item: resolved_listitem,
                        access: PathAccessFormat::new_from_pathaccessdata(current_access),
                        parent: parent_access.map(|x| PathAccessFormat::new_from_pathaccessdata(x)),
                        plugin_type,
                        title_rendered,
                        media_url,
                        rendered_comment,
                    };*/
                    //Template::render("plugin_media", data)
                    //TODO:
                    let media_type = if let Some(t) = resolved_listitem.category {
                        t
                    } else {
                        plugin_type
                    };

                    format_standard_page(
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
                                                a href=(get_media_link_resolved_url(
                                                    &media_base_url,
                                                    &current_access.path,
                                                    current_access.input.clone(),
                                                    &{
                                                        let mut parent = current_access_without_static.clone();
                                                        parent.config.language_order.value = vec![language.clone()];
                                                        parent
                                                    },
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
                                    p { b { "title" } " : " (title) }
                                }
                                @if let Some(album) = resolved_listitem.info.album {
                                    p { b { "album" } " : " (album) }
                                }
                                @if let Some(artist) = resolved_listitem.info.artist {
                                    p { b { "artist" } " : " (artist) }
                                }
                                @if let Some(year) = resolved_listitem.info.year {
                                    p { b { "year" } " : " (year) }
                                }
                                @if let Some(plot) = resolved_listitem.info.plot {
                                    p { b { "plot" } " : " (plot) }
                                }
                                @if let Some(genre) = resolved_listitem.info.genre {
                                    p { b { "genre" } " : " (genre) }
                                }
                                @if let Some(audio_language) = resolved_listitem.stream_info.audio.language {
                                    p { b { "audio language" } " : " (audio_language)}
                                }
                                @if let Some(comment) = rendered_comment {
                                    p { PreEscaped { (comment) }}
                                }
                            }

                            h2 { "download :"}

                            ul class="download" {
                                li {
                                    a href=(media_url) {
                                        @if media_type == "video" {
                                            "video"
                                        } @else if media_type == "audio" {
                                            "audio"
                                        } @else {
                                            "media"
                                        }
                                    }
                                }
                            }
                        ),
                        Some(footer),
                    )
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
                    format_standard_page(
                        title_rendered,
                        html!(
                            ul class="list_media" {
                                @for (loop_nb, sub_content) in page.sub_content.drain(..).enumerate() {
                                    li class="media_in_list" {
                                        a href=(get_absolute_plugin_path(&PathAccessData::new(sub_content.url.clone(), None, UserConfig::new_empty()), Some(&current_access))) {
                                            div class="subelem_title" { PreEscaped { (sub_content.listitem.get_display_html()) } }
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

                                        @if sub_content.listitem.is_playable() {
                                            @if plugin_type == "audio" {
                                                (contain_playable_element = true; "")
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
                            }

                            @if contain_playable_element {
                                @if plugin_type == "audio" {
                                    button id = "play_all" { "play all music syncronously (take care, may be loud and/or laggy)"}
                                    script type="text/javascript" src="/static/musicplayer.js" {}
                                }
                            }
                        ),
                        Some(footer),
                    )
                }
            }
        }
        Ok(KodiResult::Keyboard(keyboard)) => format_standard_page(
            html!("input required"),
            html!(
                p { "the plugin asked for a value"}
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
                    p { b { "default input" } " : " (default) }
                }
            ),
            None,
        ),
        Err(err) => {
            error!(
                "error while getting url \"{}\": {:?}",
                current_access.path, err
            );
            generate_error_page(html!((format!("{}", err))))
        }
    }
}
