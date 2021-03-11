//TODO: use env_logger
use clap::{App, Arg, SubCommand};
use kodi_recurse::AppArgument;
use kodi_rust::{Kodi, PathAccessData, Setting};

use console::style;
use std::collections::{HashMap, HashSet};
use std::fs::File;

use kodi_recurse::do_check;
use kodi_recurse::do_mirror;
use kodi_recurse::RecurseOption;

use indicatif::ProgressBar;

fn main() /* -> ExitCode */ { //TODO: use once stabilized
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
        .subcommand(
            SubCommand::with_name("mirror")
                .about("mirror a path and their child, recursively")
                .arg(
                    Arg::with_name("dest-path")
                        .long("dest-path")
                        .short("d")
                        .help("the destination folder")
                        .takes_value(true)
                        .required(true)
                )
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
                    return
                }
            },
            Err(err) => {
                println!(
                    "can't open the setting file at {} due to {:?}",
                    setting_path, err
                );
                return
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
            }
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
            ("mirror", Some(mirror_m)) => Some(Box::new(AppArgument {
                command_name: "mirror".into(),
                args_order: vec!["dest-path"],
                short_version: {
                    let mut h = HashMap::new();
                    h.insert("dest-path", "d");
                    h
                },
                bool_set: HashSet::new(),
                args: {
                    let mut a = HashMap::new();
                    for key in &["dest-path"] {
                        if let Some(value) = mirror_m.value_of(key) {
                            a.insert(key.to_string(), value.to_string());
                        };
                    }
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
                return
            }
        },
        None => 1,
    }; //TODO: default to one for mirror (assuming the majority of time is took by fetching)

    let plugin_path = app_argument.value_of("path").unwrap();

    let no_catch_output = app_argument.is_present("no-catch-output");

    let kodi = {
        let mut k = Kodi::new(&setting.kodi_path, u64::MAX, 200);
        k.set_catch_stdout(!no_catch_output);
        k.allowed_path = setting.allowed_path.clone();
        k.sandbox_call(app_argument.is_present("use-sandbox"));
        k
    };

    let progress_bar = if no_catch_output {
        None
    } else {
        Some(ProgressBar::new(1))
    };

    let have_progress_bar = progress_bar.is_some();

    let option = RecurseOption {
        kodi,
        top_access: PathAccessData::new(
            plugin_path.to_string(),
            None,
            setting.default_user_config.clone(),
        ),
        top_parent: app_argument
            .value_of("parent-path")
            .map(move |x| PathAccessData::new(x.to_string(), None, setting.default_user_config)),
        keep_going: app_argument.is_present("keep-going"),
        progress_bar,
        thread_nb: jobs,
        app_argument: app_argument.clone(),
    };

    //TODO: move the call to kodi_recurse out of here, just set the two variable to function
    let result = if let Some(ref sub_argument) = app_argument.sub_command {
        match sub_argument.command_name.as_str() {
            "check" => do_check(app_argument.clone(), *sub_argument.clone(), option),
            "mirror" => do_mirror(app_argument.clone(), *sub_argument.clone(), option),
            _ => panic!("an unexpected sub command was found (this is a bug)"),
        }
    } else {
        println!("no sub-command given");
        return
    };

    if !result.is_empty() {
        println!("error happened while recursing:");
        if !have_progress_bar {
            for r in &result {
                println!("");
                r.pretty_print(&app_argument);
            }
        }
    };

    if result.len() > 1 {
        println!(
            "there are {} errors.",
            style(result.len().to_string()).red()
        );
    } else if result.len() == 1 {
        println!("there is {} error.", style("one").red());
    } else {
        println!("there are no error.");
    }
}
