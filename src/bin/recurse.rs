#![feature(process_exitcode_placeholder)]
//TODO: use env_logger
use clap::{App, Arg, SubCommand};
use kodionline::recurse::kodi_recurse_par;
use kodionline::{Kodi, PathAccessData, Setting};
use reqwest::{blocking::ClientBuilder, StatusCode};
use std::fs::File;
use std::process::ExitCode;
use std::sync::Arc;

fn main() -> ExitCode {
    let app_m = App::new("kodi recurse")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .help("path to the setting file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("kodi_path")
                .short("r")
                .long("kodi_path")
                .help("path to kodi root directory")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("path")
                .short("p")
                .long("path")
                .help("the path to recurse into")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("jobs")
                .short("j")
                .long("jobs")
                .help("the max number of running thread")
                .default_value("1")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("keep_going")
                .short("k")
                .long("keep_going")
                .help("still continue the recursive browsing even when an error occurred"),
        )
        .subcommand(
            SubCommand::with_name("check")
                .about("check the validity of the given kodi path and their child")
                .arg(Arg::with_name("check_media").long("check_media").help(
                    "check the media in resolved listitem. Will produce more network request.",
                )),
        )
        .get_matches();

    let mut setting = match app_m.value_of("config") {
        Some(setting_path) => match File::open(setting_path) {
            Ok(file) => match serde_json::from_reader(file) {
                Ok(v) => v,
                Err(err) => {
                    println!(
                        "can't parse the setting file at {} due to {:?}",
                        setting_path, err
                    );
                    return ExitCode::FAILURE;
                }
            },
            Err(err) => {
                println!(
                    "can't open the setting file at {} due to {:?}",
                    setting_path, err
                );
                return ExitCode::SUCCESS;
            }
        },
        None => Setting::default(),
    };

    if let Some(kodi_path) = app_m.value_of("kodi_path") {
        setting.kodi_path = kodi_path.to_string();
    };

    let jobs = match app_m.value_of("jobs") {
        Some(jobs_str) => match jobs_str.parse() {
            Ok(v) => v,
            Err(_) => {
                println!("impossible to parse the number {}", jobs_str);
                return ExitCode::FAILURE;
            }
        },
        None => 1,
    };

    let plugin_path = app_m.value_of("path").unwrap();

    let access = PathAccessData::new(plugin_path.to_string(), None, setting.default_user_config);

    let kodi = Kodi::new(&setting.kodi_path, u64::MAX, 200);

    let keep_going = app_m.is_present("keep_going");

    let result = match app_m.subcommand() {
        ("check", Some(check_m)) => {
            //TODO: more log
            let check_media = check_m.is_present("check_media");
            let client = Arc::new(ClientBuilder::new().referer(false).build().unwrap());
            kodi_recurse_par::<(), _, _>(
                kodi,
                access,
                (),
                move |info, _| {
                    let page = info.get_page();
                    if let Some(resolved_listitem) = &page.resolved_listitem {
                        if check_media {
                            if let Some(media_url) = &resolved_listitem.path {
                                let resp = client.clone().get(media_url).send().unwrap();
                                match resp.status() {
                                    StatusCode::OK => (),
                                    err_code => info.add_error_string(format!(
                                        "getting the media at {:?} returned the error code {}",
                                        media_url, err_code
                                    )),
                                };
                            };
                        };
                        if let Some(sub_content_from_parent) = info.sub_content_from_parent {
                            if !sub_content_from_parent.listitem.is_playable() {
                                info.add_error_string("the data is not marked as playable by one of it parent, but it contain a resolved listitem".to_string());
                            };
                        };
                    } else {
                        if let Some(sub_content_from_parent) = info.sub_content_from_parent {
                            if sub_content_from_parent.listitem.is_playable() {
                                info.add_error_string("the data is marked as playable by one of it parent, but doesn't contain a resolved listitem".to_string());
                            };
                        };
                    };
                    ()
                },
                |_, _| false,
                jobs,
                keep_going,
            )
        }
        _ => {
            println!("no sub-command given");
            return ExitCode::FAILURE;
        }
    };

    //TODO: pretty print
    if result.len() > 0 {
        println!("error happended while recursing:");
        for r in &result {
            r.pretty_print();
        }
    }

    ExitCode::SUCCESS
}
