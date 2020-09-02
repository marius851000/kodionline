use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

use subprocess::{Exec, ExitStatus};

use tempfile::tempdir;

use cached::Cached;
use cached::TimedCache;

use log::error;

use crate::{data::KodiResult, PathAccessData};

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
    cache: Mutex<TimedCache<PathAccessData, KodiResult>>,
    python_command: String,
    cache_time: u64,
    cache_size: usize,
}

impl Clone for Kodi {
    fn clone(&self) -> Self {
        Self {
            kodi_config_path: self.kodi_config_path.clone(),
            cache: Mutex::new(TimedCache::with_lifespan_and_capacity(
                self.cache_time,
                self.cache_size,
            )),
            python_command: self.python_command.clone(),
            cache_time: self.cache_time,
            cache_size: self.cache_size,
        }
    }
}

impl Kodi {
    /// create a new kodi addon based on a path, with a cache configured for ``cache_time`` seconds with ``cache_elem`` cached element
    /// # Examples
    ///
    /// ```
    /// use kodionline::Kodi;
    /// let kodi = Kodi::new("~/.kodi", 3600, 500);
    /// ```
    pub fn new(path: &str, cache_time: u64, cache_size: usize) -> Self {
        Self {
            kodi_config_path: shellexpand::tilde(path).into(),
            cache: Mutex::new(TimedCache::with_lifespan_and_capacity(
                cache_time, cache_size,
            )),
            python_command: "python2".into(),
            cache_time,
            cache_size,
        }
    }

    pub fn set_python_command(&mut self, command: String) {
        self.python_command = command;
    }

    fn get_commands(&self, tempory_file: &str, access: &PathAccessData) -> Vec<String> {
        let mut result = vec![
            self.python_command.clone(),
            "kodi_interface.py".into(),
            self.kodi_config_path.clone(),
            access.path.clone(),
            tempory_file.into(),
        ];
        for input in &access.input {
            result.push("-I".into());
            result.push(input.clone());
        }
        for (add_list_key, values) in &[
            ("language_order", &access.config.language_order),
            ("resolution_order", &access.config.resolution_order),
            ("format_order", &access.config.format_order),
        ] {
            for v in *values {
                result.push("-AL".into());
                result.push(add_list_key.to_string());
                result.push(v.clone());
            }
        }
        result
    }

    /// Get the data for a kodi addon path.
    ///
    /// It will use the kodi-dl library to do this, and will sandbox the call (not actually implemented)
    ///
    /// this function also use a timed cache, that will remove element older than the time specified at initialisation.
    ///
    /// If the plugin want to get user input, you can pass a vec to expected_input that contain all the input (in the form of a string)
    /// # Errors
    /// this function return a [`KodiError`] when an error occur. there may be multiple kind of error, the most important one [`KodiError::CallError`] for when the addon crashed.
    pub fn invoke_sandbox(&self, access: &PathAccessData) -> Result<KodiResult, KodiError> {
        match self.cache.lock() {
            Ok(mut cache) => {
                if let Some(cached_value) = cache.cache_get(&access) {
                    return Ok(cached_value.clone());
                }
            }
            Err(err) => error!("the cache lock is poisoned: {:?}", err),
        };

        //TODO: make this use the sandbox
        let tempory_folder = match tempdir() {
            Ok(value) => value,
            Err(err) => return Err(KodiError::CantCreateTemporyDir(err)),
        };

        let mut data_file: PathBuf = tempory_folder.path().into(); // don't use into_path() to don't persist it
        data_file.push("tmp.json");

        let command_argument_vec = self.get_commands(&data_file.to_string_lossy(), &access);
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

        let result: KodiResult = match serde_json::from_reader(json_file) {
            Ok(value) => value,
            Err(err) => return Err(KodiError::CantParseResultFile(err)),
        };

        match self.cache.lock() {
            Ok(mut cache) => {
                cache.cache_set(access.clone(), result.clone());
            }
            Err(err) => error!("the cache lock is poisoned: {:?}", err),
        };

        Ok(result)
    }
}
