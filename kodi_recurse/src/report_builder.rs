use crate::ReportKind;

#[derive(Clone, Debug)]
pub struct ReportBuilder {
    pub summary: String,
    pub kind: ReportKind,
    pub tips: Vec<String>,
    pub logs: Vec<(String, Vec<String>)>,
    pub is_internal_error: bool,
}

impl ReportBuilder {
    pub fn new_with_summary_and_kind(summary: String, kind: ReportKind) -> Self {
        Self {
            summary,
            kind,
            tips: Vec::new(),
            logs: Vec::new(),
            is_internal_error: false,
        }
    }

    pub fn new_error(summary: String) -> Self {
        Self::new_with_summary_and_kind(summary, ReportKind::Error)
    }

    pub fn new_warning(summary: String) -> Self {
        Self::new_with_summary_and_kind(summary, ReportKind::Warning)
    }

    pub fn new_info(summary: String) -> Self {
        Self::new_with_summary_and_kind(summary, ReportKind::Info)
    }

    pub fn set_internal_error(mut self, is_internal_error: bool) -> Self {
        self.is_internal_error = is_internal_error;
        self
    }

    pub fn add_tip(mut self, tip: String) -> Self {
        self.tips.push(tip);
        self
    }

    pub fn add_log(mut self, name: String, log: Vec<String>) -> Self {
        self.logs.push((name, log));
        self
    }
}
