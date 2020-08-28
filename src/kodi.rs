use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use std::io;

use std::fs::File;

use subprocess::{Exec, ExitStatus};

use tempfile::tempdir;

use cached::Cached;
use cached::TimedCache;

use std::sync::Mutex;

use crate::data::Page;

#[derive(Debug)]
/// represent error that can happen while handling [`Kodi`]
pub enum KodiError {
    CallError(ExitStatus),
    InvalidGeneratedCommand,
    CantCreateTemporyDir(io::Error),
    CantOpenResultFile(io::Error),
    CantParseResultFile(serde_json::Error),
}

impl fmt::Display for KodiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CallError(status) => write!(
                f,
                "kodi returned with the status {:?}. Maybe the url is invalid, or can't be emulated. If you followed a link on this site, contact the webmaster with the source url and this url.",
                status
            ),
            Self::InvalidGeneratedCommand => write!(f, "internal error: the generated command is invalid"),
            Self::CantCreateTemporyDir(_) => write!(f, "internal error: can't create a tempory folder"),
            Self::CantOpenResultFile(_) => write!(f, "internal error: can't open the result file"),
            Self::CantParseResultFile(_) => write!(f, "internal error: can't parse the result file"),
        }
    }
}

impl Error for KodiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CantCreateTemporyDir(err) => Some(err),
            Self::CantOpenResultFile(err) => Some(err),
            Self::CantParseResultFile(err) => Some(err),
            _ => None,
        }
    }
}

#[derive(Debug)]
/// represent a kodi/xbmc instance. Each [`Kodi`] instance have a configuration file associated,
/// where kodi store various data, including plugin.
pub struct Kodi {
    kodi_config_path: String,
    cache: Mutex<TimedCache<String, Page>>,
    python_command: String,
}

impl Kodi {
    /// create a new kodi addon based on a path, with a cache configured for ``cache_time`` seconds with ``cache_elem`` cached element
    /// # Examples
    ///
    /// ```
    /// use kodionline::Kodi;
    /// let kodi = Kodi::new("~/.kodi", 3600, 500).unwrap();
    /// ```
    pub fn new(path: &str, cache_time: u64, cache_size: usize) -> Result<Self, KodiError> {
        Ok(Self {
            kodi_config_path: shellexpand::tilde(path).into(),
            cache: Mutex::new(TimedCache::with_lifespan_and_capacity(
                cache_time, cache_size,
            )),
            python_command: "python2".into(),
        })
    }

    pub fn set_python_command(&mut self, command: String) {
        self.python_command = command;
    }

    fn get_commands(&self, plugin_path: &str, tempory_file: &str) -> Vec<String> {
        vec![
            self.python_command.clone(),
            "kodi_interface.py".into(),
            self.kodi_config_path.clone(),
            plugin_path.into(),
            tempory_file.into(),
        ]
    }

    /// Get the data for a kodi addon path.
    ///
    /// the ``plugin_path`` should be under the form ``plugin://<plugin_id>/<url>``. ``plugin_id`` may be, for example, ``plugin.video.youtube``.
    ///
    /// It will use the kodi-dl library to do this, and will sandbox the call (not actually implemented)
    ///
    /// this function also use a timed cache, that will remove element older than the time specified at initialisation.
    ///
    /// # Errors
    /// this function return a [`KodiError`] when an error occur. there may be multiple kind of error, the most important one [`KodiError::CallError`] for when the addon crashed.
    pub fn invoke_sandbox(&self, plugin_path: &str) -> Result<Page, KodiError> {
        match self.cache.lock() {
            Ok(mut cache) => {
                if let Some(cached_value) = cache.cache_get(&plugin_path.to_string()) {
                    return Ok(cached_value.clone());
                }
            }
            Err(err) => println!("the cache lock is poisoned: {:?}", err),
        };

        //TODO: make this use the sandbox
        let tempory_folder = match tempdir() {
            Ok(value) => value,
            Err(err) => return Err(KodiError::CantCreateTemporyDir(err)),
        };

        let mut data_file: PathBuf = tempory_folder.path().into(); // don't use into_path() to don't persist it
        data_file.push("tmp.json");

        let command_argument_vec = self.get_commands(plugin_path, &data_file.to_string_lossy());
        let mut command_argument = command_argument_vec.iter();

        let first_command = match command_argument.next() {
            Some(value) => value,
            None => return Err(KodiError::InvalidGeneratedCommand),
        };
        let mut callable_command = Exec::cmd(first_command);

        for argument in command_argument {
            callable_command = callable_command.arg(argument)
        }

        let output_code = callable_command.join().unwrap();
        match output_code {
            ExitStatus::Exited(0) => (),
            other => return Err(KodiError::CallError(other)),
        }

        let json_file = match File::open(&data_file) {
            Ok(value) => value,
            Err(err) => return Err(KodiError::CantOpenResultFile(err)),
        };

        let result: Page = match serde_json::from_reader(json_file) {
            Ok(value) => value,
            Err(err) => return Err(KodiError::CantParseResultFile(err)),
        };

        match self.cache.lock() {
            Ok(mut cache) => {
                cache.cache_set(plugin_path.to_string(), result.clone());
            }
            Err(err) => println!("the cache lock is poisoned: {:?}", err),
        };

        Ok(result)
    }
}
