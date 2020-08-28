#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use rocket::config::Environment;
use rocket::State;

use serde::{Deserialize, Serialize};

use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use kodionline::data;
use kodionline::Kodi;

use std::io;

use rocket::http::uri::Uri;

use std::fs::File;

use rayon::prelude::*;

use rocket::response::{self, Redirect, Responder};
use rocket::request::Request;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Setting {
    plugins_to_show: Vec<(String, String)>,
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

#[derive(Serialize)]
struct PageRenderPlugin {
    page: data::Page,
    data_url: String,
    plugin_type: String,
    display_text: Option<String>,
    rendered_title: Vec<String>,
}

//TODO: make some merge between the folder and media part (including getting label from parent)
#[get("/plugin?<path>&<parent_path>")]
fn render_plugin(kodi: State<Kodi>, path: String, parent_path: Option<String>) -> Template {
    let path = html_escape::decode_html_entities(&path).to_string();
    let mut splited = path.split('.');
    splited.next();
    let plugin_type = match splited.next() {
        Some(value) => value.to_string(),
        None => return generate_error_page("impossible to get type of extension".to_string()),
    };

    match kodi.invoke_sandbox(&path) {
        Ok(mut page) => {
            match page.resolved_listitem.as_mut() {
                Some(mut resolved_listitem) => {
                    if let Some(parent_path) = parent_path {
                        let parent_path =
                            html_escape::decode_html_entities(&parent_path).to_string();
                        match kodi.invoke_sandbox(&parent_path) {
                            Ok(value) => {
                                for sub_content in value.sub_content {
                                    if sub_content.url == path {
                                        resolved_listitem.extend(sub_content.listitem.clone());
                                    };
                                    break;
                                }
                            }
                            Err(err) => {
                                println!(
                                    "got {:?} while trying to get the parent path {}",
                                    err, parent_path
                                );
                            }
                        }
                    };
                    // contain a media (prefered over mediatype)
                    let url = match &resolved_listitem.path {
                        Some(url) => {
                            choose_local_or_external_media_url(url.to_string(), path.as_str())
                        }
                        None => return generate_error_page("no media found for this page".into()),
                    };
                    let display_text = Some(resolved_listitem.get_display_html());
                    resolved_listitem.path = Some(url);
                    let data = PageRenderPlugin {
                        page,
                        data_url: path,
                        plugin_type,
                        display_text,
                        rendered_title: Vec::new(),
                    };
                    Template::render("plugin_media", data)
                }
                None => {
                    // contain a folder
                    let rendered_title = page
                        .sub_content
                        .iter()
                        .map(|content| content.listitem.get_display_html())
                        .collect();
                    let data = PageRenderPlugin {
                        page,
                        data_url: path,
                        plugin_type,
                        display_text: None,
                        rendered_title,
                    };
                    Template::render("plugin_folder", data)
                }
            }
        }
        Err(err) => {
            println!("error while getting url \"{}\": {:?}", path, err);
            generate_error_page(format!("{}", err))
        }
    }
}

fn choose_local_or_external_media_url(distant_url: String, path: &str) -> String {
    match distant_url.chars().next() {
        Some(first_char) => {
            if first_char == '/' {
                format!("/media?path={}", Uri::percent_encode(&path))
            } else {
                distant_url
            }
        }
        None => "".into(),
    }
}

#[derive(Serialize)]
struct PageMusicPlayer {
    musics: Vec<(String, String)>, //name, local (false) or online (true), plugin_path/url
}

#[get("/musicplayer?<path>")]
fn render_musicplayer(kodi: State<Kodi>, path: String) -> Template {
    let possible_playable_value: Vec<String> = vec![
        "isPlayable".into(),
        "IsPlayable".into(),
        "isplayable".into(),
    ];

    let path = html_escape::decode_html_entities(&path).to_string();
    match kodi.invoke_sandbox(&path) {
        Ok(media_list) => {
            fn failed_music() -> (String, String) {
                ("loading failed".into(), "".into())
            };
            let musics = media_list
                .sub_content
                .par_iter()
                // check if the media is playable
                .filter(|sub_media| {
                    let mut isplayable = false;
                    //TODO: implement this test as ListItem.is_playable
                    for isplayable_key in &possible_playable_value {
                        if let Some(isplayable_value) =
                            sub_media.listitem.properties.get(isplayable_key)
                        {
                            if isplayable_value == "true" {
                                isplayable = true;
                                break;
                            }
                        };
                    }
                    isplayable
                })
                .map(|sub_media| {
                    //TODO: check if path is already known
                    //TODO: check if media_url it point to an url or to a plugin://
                    match kodi.invoke_sandbox(&sub_media.url) {
                        Ok(submedia_loaded) => match submedia_loaded.resolved_listitem {
                            Some(mut resolved_listitem) => {
                                resolved_listitem.extend(sub_media.listitem.clone());
                                match &resolved_listitem.path {
                                    Some(path) => (
                                        resolved_listitem.get_display_html(),
                                        choose_local_or_external_media_url(
                                            path.to_string(),
                                            sub_media.url.as_str(),
                                        ),
                                    ),
                                    None => failed_music(),
                                }
                            }
                            None => failed_music(),
                        },
                        Err(_) => failed_music(),
                    }
                })
                .collect();
            let data = PageMusicPlayer { musics };
            Template::render("musicplayer", data)
        }
        Err(err) => generate_error_page(format!("{}", err)),
    }
}

#[get("/media?<path>")]
fn serve_local_media(kodi: State<Kodi>, path: String) -> Option<File> {
    //TODO: check it is in permitted area
    let path = html_escape::decode_html_entities(&path).to_string();
    match kodi.invoke_sandbox(&path) {
        Ok(media_list) => match media_list.resolved_listitem {
            Some(resolved_listitem) => match resolved_listitem.path {
                Some(data_path) => {
                    match File::open(data_path) {
                        Ok(value) => Some(value),
                        Err(err) => {
                            println!("error: can't open a file due to {:?}", err); //TODO: make this more visible. It shouldn't happen at normal time
                            None
                        }
                    }
                }
                None => None,
            },
            None => None,
        },
        Err(_) => None,
    }
}

enum MediaResponse {
    Redirect(Redirect)
}

impl<'r> Responder<'r> for MediaResponse {
    fn respond_to(self, request: &Request) -> response::Result<'r> {
        match self {
            Self::Redirect(r) => r.respond_to(request)
        }
    }
}

//TODO: merge this with server_local_media
#[get("/get_media?<path>")]
fn redirect_media(kodi: State<Kodi>, path: String) -> Option<MediaResponse> {
    let path = html_escape::decode_html_entities(&path).to_string();
    match kodi.invoke_sandbox(&path) {
        Ok(media_data) => match media_data.resolved_listitem {
            Some(resolved_listitem) => match resolved_listitem.path {
                Some(path) => {
                    Some(MediaResponse::Redirect(Redirect::to(path)))
                },
                None => None,
            },
            None => None,
        },
        Err(err) => {
            println!("error {:?} while serving {}", err, path);
            None
        },
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

    let mut kodi = Kodi::new(setting.kodi_path.clone()).unwrap();
    kodi.set_python_command(setting.python_command.clone());
    kodi.set_use_cache(!Environment::active().unwrap().is_dev());

    rocket::ignite()
        .manage(kodi)
        .manage(setting)
        .attach(Template::fairing())
        .mount(
            "/",
            routes![
                render_index,
                render_plugin,
                render_musicplayer,
                serve_local_media,
                redirect_media
            ],
        )
        .mount("/static", StaticFiles::from("static"))
        .launch();
}
