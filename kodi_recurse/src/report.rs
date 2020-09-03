use crate::AppArgument;
use console::{style, Style};
use kodi_rust::{KodiError, PathAccessData};
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
    ThreadPanicked(PathAccessData, Option<PathAccessData>),       //child, parent
    KodiCallError(PathAccessData, Arc<KodiError>),                //child, error
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
        format!(
            "{}: {}",
            report_type
                .get_tag_style()
                .apply_to(report_type.get_tag_text()),
            report_type
                .get_secondary_style()
                .apply_to(self.get_summary_text())
        )
    }

    pub fn get_summary_text(&self) -> String {
        match self {
            RecurseReport::CalledReport(_, _, message) => message.clone(),
            RecurseReport::KodiCallError(_, kodi_error) => {
                format!("can't get data from a plugin:\n{}", kodi_error)
            } //TODO: log with reason or something like that once finished, and custom display (or custom display for website ?)
            RecurseReport::ThreadPanicked(_, _) => "a thread panicked unexpectingly".into(),
        }
    }

    pub fn is_internal_error(&self) -> bool {
        // assume true by defaumt
        match self {
            Self::CalledReport(_, _, _) => false,
            Self::KodiCallError(_, call_error) => match call_error.as_ref() {
                KodiError::CallError(e) => !e.is_python_error(),
                _ => true,
            },
            _ => true,
        }
    }
    pub fn get_tip(&self, app_argument: &AppArgument) -> Vec<String> {
        let mut tips = Vec::new();

        if self.is_internal_error() {
            tips.push("this is likely an issue in the kodionline program".to_string())
        }

        //TODO: parent

        // try to get the command to reproduce the error
        let (child, parent) = self.get_reproduce_access();
        let mut new_command = app_argument.clone();
        new_command.bool_set.remove("keep-going");
        new_command.bool_set.insert("no-catch-io".into());
        new_command.args.insert("jobs".to_string(), "1".to_string());
        new_command
            .args
            .insert("path".to_string(), child.path.clone());
        if let Some(parent) = parent {
            new_command
                .args
                .insert("parent-path".to_string(), parent.path.clone());
        } else {
            new_command.args.remove("parent-path");
        };
        tips.push(format!(
            "to reproduce this error, run : {}",
            style(new_command.get_command_safe()).blue()
        ));

        tips
    }

    pub fn get_reproduce_access(&self) -> (PathAccessData, Option<PathAccessData>) {
        // child, parent if necessary/known
        match self {
            RecurseReport::CalledReport(child, parent, _) => (child.clone(), parent.clone()),
            RecurseReport::KodiCallError(child, _) => (child.clone(), None),
            RecurseReport::ThreadPanicked(child, parent) => (child.clone(), parent.clone()),
        }
    }

    pub fn get_text_to_print(&self, app_argument: &AppArgument) -> String {
        fn add_new_line(
            tag: &str,
            tag_style: &Style,
            message: &str,
            message_style: &Style,
        ) -> String {
            format!(
                "{}{}{}",
                NEWLINESPACE,
                tag_style.apply_to(format!("{}: ", tag)),
                message_style.apply_to(message)
            )
        }

        let default_style = Style::new();

        let mut string_lines = Vec::new();
        string_lines.push(format!("{}", self.get_summary_formatted()));

        let mut logs = self.get_logs();
        if logs.len() > 0 {
            //TODO: use a library to display the bar (and with color)
            for app_log in &mut logs {
                string_lines.push(format!("{}: -------", app_log.0));
                for log_line in app_log.1.drain(..) {
                    string_lines.push(log_line);
                }
                string_lines.push("------------".to_string());
            }
        }

        for tip in self.get_tip(app_argument) {
            string_lines.push(add_new_line(
                "tip",
                &Style::new().blue(),
                &tip,
                &default_style,
            ));
        }

        string_lines.join("\n")
    }

    pub fn get_logs(&self) -> Vec<(String, Vec<String>)> {
        let mut result = Vec::new();
        match self {
            Self::KodiCallError(_, err) => {
                if let KodiError::CallError(e) = err.as_ref() {
                    if let Some(log) = e.get_log() {
                        let mut collected: Vec<&str> = log.split("\n").collect::<Vec<&str>>();
                        let mut logs = Vec::new();
                        let mut all_line_included = true;

                        while let Some(last) = collected.last() {
                            if last != &"" {
                                break;
                            }
                            collected.remove(collected.len() - 1);
                        }

                        while collected.len() > 20 {
                            collected.remove(0);
                            all_line_included = false;
                        }

                        let number_of_log_line = collected.len();
                        for l in &collected {
                            logs.push(l.to_string());
                        }
                        let message = if logs.is_empty() {
                            "the addon had no log".into()
                        } else {
                            if all_line_included {
                                if logs.len() == 1 {
                                    "the onle log line".into()
                                } else {
                                    format!("all the {} log lines", number_of_log_line)
                                }
                            } else {
                                format!("lasts {} log lines", number_of_log_line)
                            }
                        };
                        result.push((message, logs));
                    }
                }
            }
            _ => (),
        };
        result
    }

    pub fn pretty_print(&self, app_argument: &AppArgument) {
        println!("{}", self.get_text_to_print(app_argument));
    }
}
