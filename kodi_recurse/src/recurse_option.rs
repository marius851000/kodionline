use kodi_rust::PathAccessData;
use indicatif::ProgressBar;
use kodi_rust::Kodi;
use crate::AppArgument;

pub struct RecurseOption {
    pub kodi: Kodi,
    pub top_access: PathAccessData,
    pub top_parent: Option<PathAccessData>,
    pub keep_going: bool,
    pub progress_bar: Option<ProgressBar>,
    pub thread_nb: usize,
    pub app_argument: AppArgument,
}
