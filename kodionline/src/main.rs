#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;

use clap::{App, Arg};
use kodi_rust::{Kodi, Setting};
use kodionline::index_page::render_index;
use kodionline::plugin_page::render_plugin;
use kodionline::redirect_page::redirect_art;
use kodionline::redirect_page::redirect_media;
use rocket::http::ContentType;
use rust_embed::Embed;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs::File;
use std::path::PathBuf;

#[derive(Embed)]
#[folder = "static/"]
struct Assets;

// from https://git.sr.ht/~pyrossh/rust-embed/tree/master/item/examples/rocket.rs
#[get("/static/<file..>")]
fn static_files(file: PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
  let filename = file.display().to_string();
  let asset = Assets::get(&filename)?;
  let content_type = file
    .extension()
    .and_then(OsStr::to_str)
    .and_then(ContentType::from_extension)
    .unwrap_or(ContentType::Bytes);

  Some((content_type, asset.data))
}

#[launch]
fn rocket() -> _{
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

    //TODO: restore with a switch in clap once clap is updated.
    let mut kodi = /*if Environment::active().unwrap().is_dev() {
        Kodi::new(&setting.kodi_path, 2, 500)
    } else {*/
        Kodi::new(&setting.kodi_path, 3600, 500);
    //};

    kodi.set_python_command(setting.python_command.clone());
    kodi.set_catch_stdout(false);
    kodi.sandbox_call(true);
    kodi.allowed_path = setting.allowed_path.clone();

    rocket::build()
        .manage(kodi)
        .manage(setting)
        .mount(
            "/",
            routes![render_index, render_plugin, redirect_media, redirect_art, static_files],
        )
}
