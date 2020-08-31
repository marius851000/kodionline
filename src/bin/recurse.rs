#![feature(process_exitcode_placeholder)]
//TODO: use env_logger
use clap::{App, Arg, SubCommand};
use kodionline::{kodi_recurse_par, Kodi, PathAccessData, Setting};
use std::fs::File;
use std::process::ExitCode;

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
                .short("k")
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
                .takes_value(true)
        )
        .subcommand(
            SubCommand::with_name("check")
                .about("check the validity of the given kodi path and their child"),
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
                return ExitCode::FAILURE
            },
        },
        None => 1,
    };

    let plugin_path = app_m.value_of("path").unwrap();

    let access = PathAccessData::new(plugin_path.to_string(), None, setting.default_user_config);

    let kodi = Kodi::new(&setting.kodi_path, u64::MAX, 200);

    match app_m.subcommand() {
        ("check", Some(_check_m)) => {
            //TODO: more log
            kodi_recurse_par::<(), _, _>(kodi, access, None, |_, _| None, |_, _| false, jobs);
        }
        _ => {
            println!("no sub-command given");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}
