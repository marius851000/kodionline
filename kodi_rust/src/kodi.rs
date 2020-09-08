use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

use subprocess::{Exec, ExitStatus, PopenError, Redirection};

use tempfile::tempdir;

use cached::Cached;
use cached::TimedCache;

use log::error;

use crate::{data::KodiResult, PathAccessData};

#[derive(Debug)]
/// represent error that can happen while handling [`Kodi`]
pub enum KodiError {
    NonZeroResult(Option<String>, ExitStatus), //log
    CantCreateProcess(PopenError),
    CantCreateTemporyDir(io::Error),
    CantOpenResultFile(io::Error),
    CantParseResultFile(serde_json::Error),
}

impl fmt::Display for KodiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonZeroResult(_, exit_status) => write!(f, "python returned a non zero value (he probably crashed, with the exit status {:?})", exit_status), //TODO: maybe better logging
            Self::CantCreateProcess(_) => write!(f, "failed to invoke the child process"),
            Self::CantCreateTemporyDir(_) => {
                write!(f, "internal error: can't create a tempory folder")
            }
            Self::CantOpenResultFile(_) => write!(f, "internal error: can't open the result file"),
            Self::CantParseResultFile(_) => {
                write!(f, "internal error: can't parse the result file")
            }
        }
    }
}

impl Error for KodiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CantCreateTemporyDir(err) => Some(err),
            Self::CantCreateProcess(err) => Some(err),
            Self::CantOpenResultFile(err) => Some(err),
            Self::CantParseResultFile(err) => Some(err),
            _ => None,
        }
    }
}

#[derive(Debug)]
/// represent a kodi/xbmc instance. Each [`Kodi`] instance have a configuration file associated,
/// where kodi store various data, including plugin.
///
/// It also have a cache to store recent results. It can be called by multiple threads.
pub struct Kodi {
    kodi_config_path: String,
    cache: Mutex<TimedCache<PathAccessData, KodiResult>>,
    python_command: String,
    cache_time: u64,
    cache_size: usize,
    catch_stdout: bool,
}

impl Kodi {
    /// create a new kodi addon based on a path, with a cache configured for ``cache_time`` seconds with ``cache_elem`` cached element
    ///
    /// # Examples
    ///
    /// ```
    /// use kodi_rust::Kodi;
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
            catch_stdout: true,
        }
    }

    /// set the command this program will use to call python. Common value should include ``python2`` and ``python3`` (default to ``python2`` until kodi 19)
    pub fn set_python_command(&mut self, command: String) {
        self.python_command = command;
    }

    /// set to ``false`` to display stdout of called python as they are computed in the terminal, or ``true`` to not display them.
    ///
    /// It the value is ``false``, it won't be able to display the program log in error message.
    pub fn set_catch_stdout(&mut self, catch_stdout: bool) {
        self.catch_stdout = catch_stdout;
    }

    fn get_arguments(&self, tempory_file: &str, access: &PathAccessData) -> Vec<String> {
        let mut result = vec![
            "kodi_interface.py".to_string(), //TODO: embed in bin, extract to tmp
            self.kodi_config_path.clone(),
            access.path.clone(),
            tempory_file.into(),
        ];
        for input in &access.input {
            result.push("-I".into());
            result.push(input.clone());
        }
        for (add_list_key, values) in &[
            ("language_order", &*access.config.language_order),
            ("resolution_order", &*access.config.resolution_order),
            ("format_order", &*access.config.format_order),
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
    ///
    /// # Errors
    ///
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

        let mut result_file: PathBuf = tempory_folder.path().into(); // don't use into_path() to don't persist it
        result_file.push("tmp.json");

        let arguments = self.get_arguments(&result_file.to_string_lossy(), &access);
        let mut to_invoke = Exec::cmd(&self.python_command);
        if self.catch_stdout {
            to_invoke = to_invoke.stdout(Redirection::Pipe)
                .stderr(Redirection::Merge);
        };

        for arg in arguments {
            to_invoke = to_invoke.arg(arg);
        };

        let (stdout, exit_status) = if self.catch_stdout {
            let captured = to_invoke.capture().map_err(|err| KodiError::CantCreateProcess(err))?;
            (Some(captured.stdout_str()), captured.exit_status)
        } else {
            (None, to_invoke.join().map_err(|err| KodiError::CantCreateProcess(err))?)
        };

        if exit_status != ExitStatus::Exited(0) {
            return Err(KodiError::NonZeroResult(stdout, exit_status));
        };

        let json_file = match File::open(&result_file) {
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
