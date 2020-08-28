#![allow(clippy::module_name_repetitions)]
mod kodi;
pub use kodi::{Kodi, KodiError};

pub mod data;

mod format;
pub use format::format_to_string;
