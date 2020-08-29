#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use rocket::config::Environment;
use rocket::State;

use serde::{Deserialize, Serialize};

use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use kodionline::{data, encode_url, should_serve_file, Kodi, format_to_string};

use rocket::request::Request;
use rocket::response::{self, NamedFile, Redirect, Responder};
use rocket::http::RawStr;

use log::{error, info};

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode, percent_decode_str};

use std::fs::File;
use std::io;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Setting {
    plugins_to_show: Vec<(String, String)>, //label, path
    kodi_path: String,
    python_command: String,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            plugins_to_show: Vec::new(),
            kodi_path: "~/.kodi".into(),
            python_command: "python3".into(),
        }
    }
}

impl Setting {
    fn get_label_for_path(&self, path: &str) -> Option<String> {
        for (label, analyzed_path) in self.plugins_to_show.iter() {
            if path == analyzed_path {
                return Some(label.clone());
            };
        }
        return None;
    }
}

fn decode_input(inputs_option: Option<&RawStr>) -> Vec<String> {
    match inputs_option {
        Some(inputs_raw) => {
            if inputs_raw.len() == 0 {
                Vec::new()
            } else {
                let mut result = Vec::new();
                for input in inputs_raw.split(":") {
                    result.push(percent_decode_str(input).decode_utf8_lossy().into()); //TODO: maybe catch the error someway, just decide what to do in this case
                };
                result
            }
        },
        None => Vec::new(),
    }
}

fn encode_input(inputs: Vec<String>) -> String {
    let mut result = String::new();
    for (input_nb, input) in inputs.iter().enumerate() {
        if input_nb != 0 {
            result.push(':');
        };
        result.extend(utf8_percent_encode(input, NON_ALPHANUMERIC).to_string().chars());
    };
    result
}

#[derive(Serialize)]
struct PageIndex {
    plugins_to_show: Vec<(String, String)>,
}

#[get("/")]
fn render_index(setting: State<Setting>) -> Template {
    let page = PageIndex {
        plugins_to_show: setting.plugins_to_show.clone(),
    };
    Template::render("index", page)
}

#[derive(Serialize)]
struct PageError {
    errormessage: String,
}

fn generate_error_page(error_message: String) -> Template {
    let data = PageError {
        errormessage: error_message,
    };
    Template::render("error", data)
}

fn get_media_link_subcontent(content: &data::SubContent) -> String {
    if let Some(media_true_url) = &content.listitem.path {
        get_media_link_resolved_url(&media_true_url, &content.url, Vec::new())
    } else {
        get_served_media_url(&content.url, Vec::new())
    }
}

fn get_media_link_resolved_url(media_url: &str, media_path: &str, input: Vec<String>) -> String {
    if should_serve_file(media_url) {
        get_served_media_url(media_path, input)
    } else {
        media_url.to_string()
    }
}

fn get_served_media_url(path: &str, input: Vec<String>) -> String {
    format!("/get_media?path={}&input={}", utf8_percent_encode(path, NON_ALPHANUMERIC), encode_input(input))
}

#[derive(Serialize)]
struct PagePluginMedia {
    item: data::ListItem,
    data_url: String,
    plugin_type: String,
    title_rendered: Option<String>,
    media_url: String,
    rendered_comment: Option<String>,
}

#[derive(Serialize)]
struct SubContentDisplay {
    data: data::SubContent,
    label_html: String,
    is_playable: bool,
    media_url: String,
}

#[derive(Serialize)]
struct PagePluginFolder {
    all_sub_content: Vec<SubContentDisplay>,
    data_url: String,
    plugin_type: String,
    title_rendered: Option<String>,
    encoded_input: String,
}

#[derive(Serialize)]
struct PagePluginKeyboard {
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
fn render_plugin(
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
    match additional_input {
        Some(value) => input.push(value),
        None => (),
    }

    let subcontent_from_parent = if let Some(ref parent_path) = parent_path {
        match kodi.invoke_sandbox(&parent_path, decode_input(parent_input)) {
            Ok(data::KodiResult::Content(parent_page)) => {
                let mut result = None;
                for sub_content in parent_page.sub_content {
                    if sub_content.url == path {
                        result = Some(sub_content);
                        break;
                    };
                }
                result
            },
            Ok(result) => {
                error!("an input was asked while asking for the parent path {} (result: {:?})", parent_path, result);
                //TODO: display input list when asked
                None
            },
            Err(err) => {
                error!(
                    "got {:?} while trying to get the parent path {}",
                    err, parent_path
                );
                None
            }
        }
    } else {
        None
    };

    match kodi.invoke_sandbox(&path, input.clone()) {
        Ok(data::KodiResult::Content(mut page)) => {
            match page.resolved_listitem {
                // contain a media
                Some(mut resolved_listitem) => {
                    if let Some(subcontent_from_parent) = subcontent_from_parent {
                        resolved_listitem.extend(subcontent_from_parent.listitem);
                    }

                    //TODO: consider redirecting to /get_media only if necessary
                    let media_url = match &resolved_listitem.path {
                        Some(url) => url.clone(),
                        None => return generate_error_page("no media found for this page".into()),
                    };

                    let title_rendered = Some(resolved_listitem.get_display_html());

                    let rendered_comment = resolved_listitem.info.comment.clone().map(|comment| format_to_string(&comment));

                    let media_url = get_media_link_resolved_url(&media_url, &path, input);

                    let data = PagePluginMedia {
                        item: resolved_listitem,
                        data_url: path,
                        plugin_type,
                        title_rendered,
                        media_url,
                        rendered_comment
                    };
                    Template::render("plugin_media", data)
                }
                // contain a folder
                None => {
                    let title_rendered = match subcontent_from_parent {
                        Some(subcontent) => Some(subcontent.listitem.get_display_html()),
                        None => setting.get_label_for_path(&path),
                    };

                    let data = PagePluginFolder {
                        all_sub_content: page
                            .sub_content
                            .drain(..)
                            .map(|content| {
                                let label_html = content.listitem.get_display_html();
                                let is_playable = content.listitem.is_playable();
                                let media_url = get_media_link_subcontent(&content);
                                SubContentDisplay {
                                    label_html,
                                    is_playable,
                                    media_url,
                                    data: content,
                                }
                            })
                            .collect(),

                        data_url: path,
                        plugin_type,
                        title_rendered,
                        encoded_input: encode_input(input)
                    };
                    Template::render("plugin_folder", data)
                }
            }
        },
        Ok(data::KodiResult::Keyboard(keyboard)) => {
            let title_rendered = match subcontent_from_parent {
                Some(subcontent) => Some(subcontent.listitem.get_display_html()),
                None => setting.get_label_for_path(&path),
            };

            let data = PagePluginKeyboard {
                plugin_type,
                data_url: path,
                title_rendered,
                parent_path: parent_path.unwrap_or("".into()),
                //TODO: replace encode_input(decode_input(...)) by clone/copy/to_string/...
                parent_input_encoded: encode_input(decode_input(parent_input)),
                input_encoded: encode_input(input),
                keyboard_default: keyboard.default.clone(),
                keyboard_hidden: keyboard.hidden,
                keyboard_heading: keyboard.heading,
            };
            Template::render("plugin_keyboard", data)
        },
        Err(err) => {
            error!("error while getting url \"{}\": {:?}", path, err);
            generate_error_page(format!("{}", err))
        }
    }
}

enum MediaResponse {
    Redirect(Redirect),
    NamedFile(NamedFile),
}

impl<'r> Responder<'r> for MediaResponse {
    fn respond_to(self, request: &Request) -> response::Result<'r> {
        match self {
            Self::Redirect(r) => r.respond_to(request),
            Self::NamedFile(f) => f.respond_to(request),
        }
    }
}

#[get("/get_media?<path>&<input>")]
fn redirect_media(kodi: State<Kodi>, path: String, input: Option<&RawStr>) -> Option<MediaResponse> {
    match kodi.invoke_sandbox(&path, decode_input(input)) {
        Ok(data::KodiResult::Content(media_data)) => match media_data.resolved_listitem {
            Some(resolved_listitem) => match resolved_listitem.path {
                Some(media_url) => {
                    if should_serve_file(&media_url) {
                        //TODO: check if the file is permitted to be read
                        Some(MediaResponse::NamedFile(match NamedFile::open(media_url) {
                            Ok(file) => file,
                            Err(err) => {
                                error!("failed to open the local file due to {:?}", err);
                                return None;
                            }
                        }))
                    } else {
                        let encoded = encode_url(&media_url);
                        info!("redirecting the media {} to \"{}\"", path, encoded);
                        Some(MediaResponse::Redirect(Redirect::to(encoded)))
                    }
                }
                None => None,
            },
            None => None,
        },
        Ok(result) => {
            error!("asked for input to access the media at {} (result: {:?})", path, result);
            None
        },
        Err(err) => {
            error!("error {:?} while serving {}", err, path);
            None
        }
    }
}

fn main() {
    let setting: Setting = match File::open("./setting.json") {
        Ok(file) => serde_json::from_reader(file).unwrap(),
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => Setting::default(),
            err => panic!(err),
        },
    };

    let mut kodi = if Environment::active().unwrap().is_dev() {
        Kodi::new(&setting.kodi_path, 2, 500).unwrap()
    } else {
        Kodi::new(&setting.kodi_path, 3600, 500).unwrap()
    };

    kodi.set_python_command(setting.python_command.clone());

    rocket::ignite()
        .manage(kodi)
        .manage(setting)
        .attach(Template::fairing())
        .mount("/", routes![render_index, render_plugin, redirect_media])
        .mount("/static", StaticFiles::from("static"))
        .launch();
}
