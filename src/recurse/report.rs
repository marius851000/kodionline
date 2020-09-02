use crate::{KodiError, PathAccessData};
use console::Style;
use std::sync::Arc;
//use shell_escape::escape;

static NEWLINESPACE: &'static str = "  ";

#[derive(Debug, Clone)]
pub enum ReportKind {
    Error,
    Warning,
    Info,
}

//mimic the rust error report style
impl ReportKind {
    pub fn get_tag_style(&self) -> Style {
        match self {
            ReportKind::Error => Style::new().red().bold(),
            ReportKind::Warning => Style::new().yellow().bold(),
            ReportKind::Info => Style::new().green().bold(),
        }
    }

    pub fn get_tag_text(&self) -> &'static str {
        match self {
            ReportKind::Error => "error",
            ReportKind::Warning => "warning",
            ReportKind::Info => "info",
        }
    }

    pub fn get_secondary_style(&self) -> Style {
        match self {
            ReportKind::Error => Style::new().bold(),
            ReportKind::Warning => Style::new().bold(),
            ReportKind::Info => Style::new().bold(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RecurseReport {
    CalledReport(PathAccessData, Option<PathAccessData>, String), //child, parent, message
    ThreadPanicked(PathAccessData, Option<PathAccessData>), //child, parent
    KodiCallError(PathAccessData, Arc<KodiError>), //child, error
}

impl RecurseReport {
    pub fn get_report_type(&self) -> ReportKind {
        match self {
            RecurseReport::CalledReport(_, _, _) => ReportKind::Error,
            RecurseReport::ThreadPanicked(_, _) => ReportKind::Error,
            RecurseReport::KodiCallError(_, _) => ReportKind::Error,
        }
    }

    pub fn get_summary_formatted(&self) -> String {
        let report_type = self.get_report_type();
        format!("{}: {}",
            report_type.get_tag_style().apply_to(report_type.get_tag_text()),
            report_type.get_secondary_style().apply_to(self.get_summary_text())
        )
    }

    pub fn get_summary_text(&self) -> String {
        match self {
            RecurseReport::CalledReport(_, _, message) => message.clone(),
            RecurseReport::KodiCallError(_, kodi_error) => format!("can't get plugin data:\n{}", kodi_error), //TODO: log with reason or something like that once finished
            RecurseReport::ThreadPanicked(_, _) => "a thread panicked unexpectingly".into(),
        }
    }

    pub fn get_tip(&self) -> Vec<String> {
        match self {
            RecurseReport::CalledReport(_, _, _) => Vec::new(),
            RecurseReport::KodiCallError(_, _) => Vec::new(),
            RecurseReport::ThreadPanicked(_, _) => vec!["this is likely an issue in the kodionline program".into()]
        }
    }

    pub fn get_reproduce_access(&self) -> (PathAccessData, Option<PathAccessData>) { // child, parent if necessary/known
        match self {
            RecurseReport::CalledReport(child, parent, _) => (child.clone(), parent.clone()),
            RecurseReport::KodiCallError(child, _) => (child.clone(), None),
            RecurseReport::ThreadPanicked(child, parent) => (child.clone(), parent.clone())
        }
    }

    pub fn get_text_to_print(&self) -> String {
        fn add_new_line(tag: &str, tag_style: &Style, message: &str, message_style: &Style) -> String {
            format!("{}{}{}", NEWLINESPACE, tag_style.apply_to(format!("{}: ", tag)), message_style.apply_to(message))
        }

        let default_style = Style::new();

        let mut string_lines = Vec::new();
        string_lines.push(format!("{}", self.get_summary_formatted()));

        for tip in self.get_tip() {
            string_lines.push(add_new_line("tip", &Style::new().blue(), &tip, &default_style));
        };


        string_lines.join("\n")
    }

    pub fn pretty_print(&self) {
        println!("{}", self.get_text_to_print());
    }
}
