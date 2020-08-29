#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use rocket::config::Environment;
use rocket::State;

use serde::{Deserialize, Serialize};

use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use kodionline::{data, encode_url, is_local_path, Kodi, format_to_string};

use rocket::request::Request;
use rocket::response::{self, NamedFile, Redirect, Responder};

use log::{error, info};

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

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
        get_media_link_resolved_url(&media_true_url, &content.url)
    } else {
        get_served_media_url(&content.url)
    }
}

fn get_media_link_resolved_url(media_url: &str, media_path: &str) -> String {
    if is_local_path(media_url) {
        get_served_media_url(media_path)
    } else {
        media_url.to_string()
    }
}

fn get_served_media_url(path: &str) -> String {
    format!("/get_media?path={}", utf8_percent_encode(path, NON_ALPHANUMERIC))
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
}

#[get("/plugin?<path>&<parent_path>")]
fn render_plugin(
    kodi: State<Kodi>,
    setting: State<Setting>,
    path: String,
    parent_path: Option<String>,
) -> Template {
    let mut splited = path.split('.');
    splited.next();
    let plugin_type = match splited.next() {
        Some(value) => value.to_string(),
        None => return generate_error_page("impossible to get type of extension".to_string()),
    };

    let subcontent_from_parent = if let Some(parent_path) = parent_path {
        match kodi.invoke_sandbox(&parent_path) {
            Ok(parent_page) => {
                let mut result = None;
                for sub_content in parent_page.sub_content {
                    if sub_content.url == path {
                        result = Some(sub_content);
                        break;
                    };
                }
                result
            }
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

    match kodi.invoke_sandbox(&path) {
        Ok(mut page) => {
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

                    let media_url = get_media_link_resolved_url(&media_url, &path);

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
                    };
                    Template::render("plugin_folder", data)
                }
            }
        }
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

#[get("/get_media?<path>")]
fn redirect_media(kodi: State<Kodi>, path: String) -> Option<MediaResponse> {
    match kodi.invoke_sandbox(&path) {
        Ok(media_data) => match media_data.resolved_listitem {
            Some(resolved_listitem) => match resolved_listitem.path {
                Some(media_url) => {
                    if is_local_path(&media_url) {
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
