use kodi_recurse::AppArgument;
use kodi_recurse::{RecurseReport, ReportKind};
use kodi_rust::{PathAccessData, UserConfig};
use std::collections::{HashMap, HashSet};

fn main() {
    println!("displaying the various report kind:");
    for kind in &[ReportKind::Error, ReportKind::Warning, ReportKind::Info] {
        println!(
            "{} : {}",
            kind.get_tag_style().apply_to(kind.get_tag_text()),
            kind.get_secondary_style()
                .apply_to(format!("sample {:?} text", kind))
        );
    }
    println!("displaying an error:");
    let access = PathAccessData::new(
        "plugin://.../?somekodiurlpath".into(),
        None,
        UserConfig::default(),
    );
    let message = RecurseReport::ThreadPanicked(access, None);

    let argument = AppArgument {
        command_name: "binary_name".into(), //TODO: a bit more of stuff to show this
        ..AppArgument::default()
    };

    message.pretty_print(&argument);
}
