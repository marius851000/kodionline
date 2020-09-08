#![feature(process_exitcode_placeholder)]
//TODO: use env_logger
use clap::{App, Arg, SubCommand};
use kodi_recurse::kodi_recurse_par;
use kodi_recurse::AppArgument;
use kodi_rust::{Kodi, PathAccessData, Setting};
use reqwest::{blocking::ClientBuilder, StatusCode};

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::process::ExitCode;
use std::sync::Arc;
use console::style;

use indicatif::ProgressBar;

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
            Arg::with_name("kodi-path")
                .short("r")
                .long("kodi-path")
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
            Arg::with_name("parent-path")
                .short("P")
                .long("parent-path")
                .help("the path of the parent in the path")
                .takes_value(true),
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
            Arg::with_name("keep-going")
                .short("k")
                .long("keep-going")
                .help("still continue the recursive browsing even when an error occurred"),
        )
        .arg(
            Arg::with_name("no-catch-output")
                .short("n")
                .long("no-catch-io")
                .help("do not catch the output of python code. It will log the python output as it is executed, rather than being displayed only after a crash")
        )
        .arg(
            Arg::with_name("use-sandbox")
                .short("s")
                .long("use-sandbox")
                .help("sandbox the called command with bubblewrap")
        )
        .subcommand(
            SubCommand::with_name("check")
                .about("check the validity of the given kodi path and their child")
                .arg(Arg::with_name("check-media").long("check-media").help(
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
                return ExitCode::FAILURE;
            }
        },
        None => Setting::default(),
    };

    let app_argument = AppArgument {
        command_name: "kodi_recurse".into(),
        args_order: vec![
            "config",
            "kodi_path",
            "path",
            "jobs",
            "parent-path",
            "no-catch-output",
            "keep_going",
            "use-sandbox",
        ],
        short_version: {
            let mut s = HashMap::new();
            s.insert("config", "c");
            s.insert("kodi-path", "r");
            s.insert("path", "p");
            s.insert("parent-path", "P");
            s.insert("jobs", "j");
            s.insert("keep-going", "k");
            s.insert("no-catch-output", "n");
            s.insert("use-sandbox", "s");
            s
        },
        args: {
            let mut a = HashMap::new();
            for parameter in &["config", "kodi-path", "path", "parent-path", "jobs"] {
                if let Some(value) = app_m.value_of(parameter) {
                    a.insert(parameter.to_string(), value.to_string());
                };
            }
            a
        },
        bool_set: {
            let mut b = HashSet::new();
            for param in &["keep-going", "no-catch-output", "use-sandbox"] {
                if app_m.is_present(param) {
                    b.insert(param.to_string());
                };
            };
            b
        },
        sub_command: match app_m.subcommand() {
            ("check", Some(check_m)) => Some(Box::new(AppArgument {
                command_name: "check".into(),
                args_order: vec!["check-media"],
                short_version: HashMap::new(),
                bool_set: HashSet::new(),
                args: {
                    let mut a = HashMap::new();
                    if let Some(check_media) = check_m.value_of("check-media") {
                        a.insert("check-media".to_string(), check_media.to_string());
                    };
                    a
                },
                sub_command: None,
            })),
            _ => None,
        },
    };

    if let Some(kodi_path) = app_argument.value_of("kodi-path") {
        setting.kodi_path = kodi_path.to_string();
    };

    let jobs = match app_argument.value_of("jobs") {
        Some(jobs_str) => match jobs_str.parse() {
            Ok(v) => v,
            Err(_) => {
                println!("impossible to parse the number {}", jobs_str);
                return ExitCode::FAILURE;
            }
        },
        None => 1,
    };

    let plugin_path = app_argument.value_of("path").unwrap();

    let access = PathAccessData::new(
        plugin_path.to_string(),
        None,
        setting.default_user_config.clone(),
    );

    let no_catch_output = app_argument.is_present("no-catch-output");

    let kodi = {
        let mut k = Kodi::new(&setting.kodi_path, u64::MAX, 200);
        k.set_catch_stdout(!no_catch_output);
        k.allowed_path = setting.allowed_path.clone();
        k.sandbox_call(app_argument.is_present("use-sandbox"));
        k
    };

    let keep_going = app_argument.is_present("keep-going");

    let parent_path = app_argument.value_of("parent-path");

    let parent = parent_path
        .map(move |x| PathAccessData::new(x.to_string(), None, setting.default_user_config));

    let progress_bar = if no_catch_output {
        None
    } else {
        Some(ProgressBar::new(1))
    };

    //TODO: move the call to kodi_recurse out of here, just set the two variable to function
    let result = match app_m.subcommand() {
        ("check", Some(check_m)) => {
            //TODO: more control on verbosity
            let check_media = check_m.is_present("check-media");
            let client = Arc::new(ClientBuilder::new().referer(false).build().unwrap());
            kodi_recurse_par::<(), _, _>(
                kodi,
                access,
                parent,
                (),
                move |info, _| {
                    let page = info.get_page();
                    if let Some(resolved_listitem) = &page.resolved_listitem {
                        if check_media {
                            // check if the resolved media exist
                            //TODO: check other referenced content, and make look help look exactly what is wrong
                            if let Some(media_url) = &resolved_listitem.path {
                                if media_url.starts_with("http://")
                                    | media_url.starts_with("http://")
                                {
                                    let resp = client.clone().get(media_url).send().unwrap();
                                    match resp.status() {
                                        StatusCode::OK => (),
                                        err_code => info.add_error_string(format!(
                                            "getting the distant media at {:?} returned the error code {}",
                                            media_url, err_code
                                        )),
                                    };
                                }
                                if media_url.starts_with("/") {
                                    if let Err(err) = File::open(media_url) {
                                        info.add_error_string(format!(
                                            "can't get the local media at {:?}: {:?}",
                                            media_url, err
                                        ));
                                    };
                                } else {
                                    info.add_error_string(format!(
                                        "can't determine how to check the existance of {:?}",
                                        media_url
                                    ));
                                }
                            };
                        };
                    };
                    // check that the IsPlatable flag is valid
                    if page.resolved_listitem.is_some() {
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
                keep_going,
                progress_bar,
                jobs,
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
            println!("");
            r.pretty_print(&app_argument);
        }
    }

    if result.len() > 1 {
        println!("there are {} errors.", style(result.len().to_string()).red());
    } else if result.len() == 1 {
        println!("there is {} error.", style("one").red());
    } else {
        println!("there are no error.");
    }

    ExitCode::SUCCESS
}
