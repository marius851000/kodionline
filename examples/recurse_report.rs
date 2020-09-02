use kodionline::recurse::report::{RecurseReport, ReportKind};
use kodionline::{PathAccessData, UserConfig};

fn main() {
    println!("displaying the various report kind:");
    for kind in &[ReportKind::Error, ReportKind::Warning, ReportKind::Info] {
        println!(
            "{} : {}",
            kind.get_tag_style().apply_to(kind.get_tag_text()),
            kind.get_secondary_style().apply_to(format!("sample {:?} text", kind))
        );
    }
    println!("displaying an error:");
    let access = PathAccessData::new("plugin://.../?somekodiurlpath".into(), None, UserConfig::default());
    let message = RecurseReport::ThreadPanicked(access, None);
    message.pretty_print();
}
