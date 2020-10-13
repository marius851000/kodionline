#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use kodionline::index_page::static_rocket_route_info_for_render_index;
use kodionline::plugin_page::static_rocket_route_info_for_render_plugin;
use kodionline::redirect_page::static_rocket_route_info_for_redirect_art;
use kodionline::redirect_page::static_rocket_route_info_for_redirect_media;
use rocket::config::Environment;
use rocket_contrib::serve::StaticFiles;

use clap::{App, Arg};

use kodi_rust::{Kodi, Setting};

use std::fs::File;

fn main() {
    let app_m = App::new("kodi online")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .help("path to the setting file")
                .takes_value(true),
        )
        .get_matches();

    let setting = if let Some(config_path) = app_m.value_of("config") {
        let file = File::open(config_path).unwrap();
        serde_json::from_reader(file).unwrap()
    } else {
        Setting::default()
    };

    let mut kodi = if Environment::active().unwrap().is_dev() {
        Kodi::new(&setting.kodi_path, 2, 500)
    } else {
        Kodi::new(&setting.kodi_path, 3600, 500)
    };

    kodi.set_python_command(setting.python_command.clone());
    kodi.set_catch_stdout(false);
    kodi.sandbox_call(true);
    kodi.allowed_path = setting.allowed_path.clone();

    rocket::ignite()
        .manage(kodi)
        .manage(setting)
        .mount(
            "/",
            routes![render_index, render_plugin, redirect_media, redirect_art],
        )
        .mount("/static", StaticFiles::from("static"))
        .launch();
}
