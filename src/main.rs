#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;
use rocket::State;

use serde::{Deserialize, Serialize};

use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use kodionline::data;
use kodionline::Kodi;

use std::io;

use rocket::http::uri::Uri;

use std::fs::File;

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
}

#[get("/plugin?<path>")]
fn render_plugin(kodi: State<Kodi>, path: String) -> Template {
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
                    // contain a media (prefered over mediatype)
                    let url = match &resolved_listitem.path {
                        Some(url) => {
                            choose_local_or_external_media_url(url.to_string(), path.as_str())
                        }
                        None => return generate_error_page("no media found for this page".into()),
                    };
                    resolved_listitem.path = Some(url);
                    let data = PageRenderPlugin {
                        page,
                        data_url: path,
                        plugin_type,
                    };
                    Template::render("plugin_media", data)
                }
                None => {
                    // contain a folder
                    let data = PageRenderPlugin {
                        page,
                        data_url: path,
                        plugin_type,
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
    let path = html_escape::decode_html_entities(&path).to_string();
    match kodi.invoke_sandbox(&path) {
        Ok(media_list) => {
            let mut musics = Vec::new();
            for sub_media in media_list.sub_content.iter() {
                if sub_media.listitem.properties["IsPlayable"] != "true" {
                    continue;
                };
                fn failed_music() -> (String, String) {
                    ("loading failed".into(), "".into())
                };
                musics.push(match &sub_media.listitem.path {
                    Some(media_url) => {
                        (sub_media.listitem.get_display_text(), media_url.to_string())
                    }
                    None => match kodi.invoke_sandbox(&sub_media.url) {
                        Ok(submedia_loaded) => match submedia_loaded.resolved_listitem {
                            Some(resolved_listitem) => match &resolved_listitem.path {
                                Some(path) => (
                                    resolved_listitem.get_display_text(),
                                    choose_local_or_external_media_url(
                                        path.to_string(),
                                        sub_media.url.as_str(),
                                    ),
                                ),
                                None => failed_music(),
                            },
                            None => failed_music(),
                        },
                        Err(_) => failed_music(),
                    },
                })
            }
            let data = PageMusicPlayer { musics };
            Template::render("musicplayer", data)
        }
        Err(err) => generate_error_page(format!("{}", err)),
    }
}

#[get("/media?<path>")]
fn server_local_media(kodi: State<Kodi>, path: String) -> Option<File> {
    //TODO: check it is in permitted area
    let path = html_escape::decode_html_entities(&path).to_string();
    println!("requested file for {:?}", path);
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

#[get("/")]
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
                server_local_media
            ],
        )
        .mount("/static", StaticFiles::from("static"))
        .launch();
}
