#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use kodionline::plugin_page::static_rocket_route_info_for_render_plugin;
use kodionline::redirect_page::static_rocket_route_info_for_redirect_art;
use kodionline::redirect_page::static_rocket_route_info_for_redirect_media;

use rocket::config::Environment;
use rocket::State;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;

use kodionline::{Kodi, Setting};

use serde::Serialize;
use std::fs::File;
use std::io;

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

fn main() {
    let setting: Setting = match File::open("./setting.json") {
        Ok(file) => serde_json::from_reader(file).unwrap(),
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => Setting::default(),
            err => panic!(err),
        },
    };

    let mut kodi = if Environment::active().unwrap().is_dev() {
        Kodi::new(&setting.kodi_path, 2, 500)
    } else {
        Kodi::new(&setting.kodi_path, 3600, 500)
    };

    kodi.set_python_command(setting.python_command.clone());

    rocket::ignite()
        .manage(kodi)
        .manage(setting)
        .attach(Template::fairing())
        .mount(
            "/",
            routes![render_index, render_plugin, redirect_media, redirect_art],
        )
        .mount("/static", StaticFiles::from("static"))
        .launch();
}
