#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use rocket::config::Environment;
use rocket::State;

use serde::{Deserialize, Serialize};

use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use kodionline::{data, is_local_path, Kodi};

use rayon::prelude::*;

use rocket::request::Request;
use rocket::response::{self, Redirect, Responder};

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

#[derive(Serialize)]
struct PageRenderPlugin {
    page: data::Page,
    data_url: String,
    plugin_type: String,
    display_text: Option<String>,
    rendered_title: Vec<String>,
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
                println!(
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
            match page.resolved_listitem.as_mut() {
                // contain a media
                Some(mut resolved_listitem) => {
                    if let Some(subcontent_from_parent) = subcontent_from_parent {
                        resolved_listitem.extend(subcontent_from_parent.listitem);
                    }

                    let url = match &resolved_listitem.path {
                        Some(url) => url.clone(),
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
                // contain a folder
                None => {
                    let rendered_title = page
                        .sub_content
                        .par_iter()
                        .map(|content| content.listitem.get_display_html())
                        .collect();

                    let display_text = match subcontent_from_parent {
                        Some(subcontent) => Some(subcontent.listitem.get_display_html()),
                        None => setting.get_label_for_path(&path),
                    };

                    let data = PageRenderPlugin {
                        page,
                        data_url: path,
                        plugin_type,
                        display_text,
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

enum MediaResponse {
    Redirect(Redirect),
    File(File),
}

impl<'r> Responder<'r> for MediaResponse {
    fn respond_to(self, request: &Request) -> response::Result<'r> {
        match self {
            Self::Redirect(r) => r.respond_to(request),
            Self::File(f) => f.respond_to(request),
        }
    }
}

#[get("/get_media?<path>")]
fn redirect_media(kodi: State<Kodi>, path: String) -> Option<MediaResponse> {
    match kodi.invoke_sandbox(&path) {
        Ok(media_data) => match media_data.resolved_listitem {
            Some(resolved_listitem) => match resolved_listitem.path {
                Some(path) => {
                    if is_local_path(&path) {
                        //TODO: check if the file is permitted to be read
                        Some(MediaResponse::File(match File::open(path) {
                            Ok(file) => file,
                            Err(err) => {
                                println!("failed to open the local file due to {:?}", err);
                                return None;
                            }
                        }))
                    } else {
                        Some(MediaResponse::Redirect(Redirect::to(path)))
                    }
                }
                None => None,
            },
            None => None,
        },
        Err(err) => {
            println!("error {:?} while serving {}", err, path);
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
