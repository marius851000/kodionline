use crate::{
    data::{KodiResult, ListItem, SubContent},
    error_page::generate_error_page,
    format_to_string, get_art_link_subcontent, get_media_link_resolved_url,
    get_media_link_subcontent, get_sub_content_from_parent,
    input::{decode_input, encode_input},
    Kodi, PathAccessData, Setting,
};

use log::error;
use rocket::{http::RawStr, State};
use rocket_contrib::templates::Template;
use serde::Serialize;

#[derive(Serialize)]
pub struct PagePluginMedia {
    item: ListItem,
    data_url: String,
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
    data_url: String,
    plugin_type: String,
    title_rendered: Option<String>,
    encoded_input: String,
}

#[derive(Serialize)]
pub struct PagePluginKeyboard {
    plugin_type: String,
    data_url: String,
    title_rendered: Option<String>,
    parent_path: String,
    parent_input_encoded: String,
    input_encoded: String,
    keyboard_default: Option<String>,
    keyboard_heading: Option<String>,
    keyboard_hidden: bool,
}

#[get("/plugin?<path>&<parent_path>&<input>&<parent_input>&<additional_input>")]
pub fn render_plugin(
    kodi: State<Kodi>,
    setting: State<Setting>,
    path: String,
    parent_path: Option<String>,
    input: Option<&RawStr>,
    parent_input: Option<&RawStr>,
    additional_input: Option<String>,
) -> Template {
    let mut splited = path.split('.');
    splited.next();
    let plugin_type = match splited.next() {
        Some(value) => value.to_string(),
        None => return generate_error_page("impossible to get type of extension".to_string()),
    };

    let mut input = decode_input(input);

    if let Some(value) = additional_input {
        input.push(value)
    }

    let current_access = PathAccessData { path, input };

    let parent_access = PathAccessData::try_create_from_url(parent_path.clone(), parent_input);

    let subcontent_from_parent = if let Some(ref parent_access_internal) = parent_access {
        get_sub_content_from_parent(&kodi, &parent_access_internal, &current_access.path)
    } else {
        None
    };

    match kodi.invoke_sandbox(&current_access.path, current_access.input.clone()) {
        Ok(KodiResult::Content(mut page)) => {
            match page.resolved_listitem {
                // contain a media
                Some(mut resolved_listitem) => {
                    if let Some(subcontent_from_parent) = subcontent_from_parent {
                        resolved_listitem.extend(subcontent_from_parent.listitem);
                    }

                    let media_url = match &resolved_listitem.path {
                        Some(url) => url.clone(),
                        None => return generate_error_page("no media found for this page".into()),
                    };

                    let title_rendered = Some(resolved_listitem.get_display_html());

                    let rendered_comment = resolved_listitem
                        .info
                        .comment
                        .clone()
                        .map(|comment| format_to_string(&comment));

                    let media_url = get_media_link_resolved_url(
                        &media_url,
                        &current_access.path,
                        current_access.input.clone(),
                        &current_access,
                    );

                    let data = PagePluginMedia {
                        item: resolved_listitem,
                        data_url: current_access.path,
                        plugin_type,
                        title_rendered,
                        media_url,
                        rendered_comment,
                    };
                    Template::render("plugin_media", data)
                }
                // contain a folder
                None => {
                    let title_rendered = match subcontent_from_parent {
                        Some(subcontent) => Some(subcontent.listitem.get_display_html()),
                        None => setting.get_label_for_path(&current_access.path),
                    };

                    let data = PagePluginFolder {
                        all_sub_content: page
                            .sub_content
                            .drain(..)
                            .map(|content| {
                                let label_html = content.listitem.get_display_html();
                                let is_playable = content.listitem.is_playable();
                                let media_url =
                                    get_media_link_subcontent(&content, &current_access);
                                let art_url = match content.listitem.get_thumb_category() {
                                    Some(art_category) => Some(get_art_link_subcontent(
                                        &content,
                                        art_category,
                                        &current_access,
                                    )),
                                    None => None,
                                };
                                SubContentDisplay {
                                    label_html,
                                    is_playable,
                                    media_url,
                                    data: content,
                                    art_url,
                                }
                            })
                            .collect(),

                        data_url: current_access.path,
                        plugin_type,
                        title_rendered,
                        encoded_input: encode_input(&current_access.input),
                    };
                    Template::render("plugin_folder", data)
                }
            }
        }
        Ok(KodiResult::Keyboard(keyboard)) => {
            let title_rendered = match subcontent_from_parent {
                Some(subcontent) => Some(subcontent.listitem.get_display_html()),
                None => setting.get_label_for_path(&current_access.path),
            };

            #[allow(clippy::or_fun_call)]
            let data = PagePluginKeyboard {
                plugin_type,
                data_url: current_access.path,
                title_rendered,
                parent_path: parent_path.unwrap_or("".to_string()),
                //TODO: replace encode_input(&decode_input(...)) by clone/copy/to_string/...
                parent_input_encoded: encode_input(&decode_input(parent_input)),
                input_encoded: encode_input(&current_access.input),
                keyboard_default: keyboard.default.clone(),
                keyboard_hidden: keyboard.hidden,
                keyboard_heading: keyboard.heading,
            };
            Template::render("plugin_keyboard", data)
        }
        Err(err) => {
            error!(
                "error while getting url \"{}\": {:?}",
                current_access.path, err
            );
            generate_error_page(format!("{}", err))
        }
    }
}
