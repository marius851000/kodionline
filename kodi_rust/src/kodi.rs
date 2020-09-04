use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use rand::distributions::Uniform;
use rand::{thread_rng, Rng};

use shell_escape::escape;
use std::borrow::Cow;
use subprocess::{Exec, ExitStatus, Popen, PopenError, Redirection};

use tempfile::tempdir;

use cached::Cached;
use cached::TimedCache;

use log::error;

use crate::{data::KodiResult, PathAccessData};

#[derive(Debug)]
/// represent error that can happen while handling [`Kodi`]
pub enum KodiError {
    CallError(KodiCallError),
    InvalidGeneratedCommand,
    CantCreateTemporyDir(io::Error),
    CantOpenResultFile(io::Error),
    CantParseResultFile(serde_json::Error),
}

impl fmt::Display for KodiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CallError(_) => write!(f, "failed to call python code"),
            Self::InvalidGeneratedCommand => {
                write!(f, "internal error: the generated command is invalid")
            }
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
            Self::CantOpenResultFile(err) => Some(err),
            Self::CantParseResultFile(err) => Some(err),
            Self::CallError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<KodiCallError> for KodiError {
    fn from(k: KodiCallError) -> Self {
        Self::CallError(k)
    }
}

#[derive(Debug)]
pub enum KodiCallError {
    NonZeroExit(String, ExitStatus),
    CantGetStdin(),
    CantWriteStdin(io::Error),
    CantGetStdout(String),
    CantReadStdout(String, io::Error),
    CreateProcessError(PopenError),
}

impl KodiCallError {
    pub fn is_python_error(&self) -> bool {
        match self {
            Self::NonZeroExit(_, _) => true,
            _ => false,
        }
    }

    pub fn get_log(&self) -> Option<&str> {
        match self {
            Self::NonZeroExit(log, _) => Some(&log),
            Self::CantGetStdout(log) => Some(&log),
            Self::CantReadStdout(log, _) => Some(&log),
            _ => None,
        }
    }
}

impl fmt::Display for KodiCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonZeroExit(_, exit_code) => write!(
                f,
                "the exit code of python is {:?} (expected normal exit code 0). Python code likely crashed.", exit_code //TODO: maybe better display of the exit code
            ),
            Self::CantGetStdin() => write!(
                f,
                "impossible to get an handle to the stdin of the invoked process"
            ),
            Self::CantWriteStdin(_) => write!(
                f,
                "impossible to write to the invoked command stdin (input)"
            ),
            Self::CantGetStdout(_) => write!(
                f,
                "impossible to get an handle to the stdout of the invoked process"
            ),
            Self::CantReadStdout(_, _) => write!(
                f,
                "impossible to read to the invoked command stdout (output)"
            ),
            Self::CreateProcessError(_) => write!(f, "impossible to spawn the child process"),
        }
    }
}

impl Error for KodiCallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CantWriteStdin(err) => Some(err),
            Self::CantReadStdout(_, err) => Some(err),
            Self::CreateProcessError(err) => Some(err),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct KodiInterface {
    process: Option<Popen>,
    exit_hash: String,
    python_command: String,
    redirect: bool,
}

impl KodiInterface {
    fn new_with_python(python_command: String, redirect: bool) -> KodiInterface {
        let exit_hash = {
            let mut rng = thread_rng();
            let valid_character_generator = Uniform::new(65, 122);
            let mut h = String::from("end of call (with check) :");
            for _ in 0..100 {
                h.push(rng.sample(valid_character_generator).into());
            }
            h
        };

        KodiInterface {
            process: None,
            exit_hash,
            redirect,
            python_command,
        }
    }

    //TODO: multiple call
    fn run(&mut self, command: &Vec<String>) -> Result<String, KodiCallError> {
        fn invoke_process(exit_hash: &str, python_command: &str) -> Result<Popen, KodiCallError> {
            Exec::cmd(python_command)
                .arg("kodi_interface.py")
                .arg(exit_hash)
                .stdin(Redirection::Pipe)
                .stdout(Redirection::Pipe)
                .stderr(Redirection::Merge)
                .popen()
                .map_err(|err| KodiCallError::CreateProcessError(err))
        }

        //TODO: this is a workaround: calling communicate_start(Some(...)) multiple time on the same process panic
        //search for a python-rust communication method not involving stdin (stdout can still be used. also, store the Communicator in the storage for reuse)
        self.process = None;
        match &mut self.process {
            None => {
                let popen = invoke_process(&self.exit_hash, &self.python_command)?;
                self.process = Some(popen);
            }
            Some(pro) => {
                if pro.poll().is_some() {
                    let popen = invoke_process(&self.exit_hash, &self.python_command)?;
                    self.process = Some(popen);
                }
            }
        };

        let process: &mut Popen = self.process.as_mut().unwrap();

        let command_escaped = command
            .iter()
            .map(|x| escape(Cow::from(x)).to_string())
            .collect::<Vec<String>>()
            .join(" ");

        let mut communicate = process
            .communicate_start(Some(format!("run {}\n", command_escaped).as_bytes().into()))
            .limit_time(Duration::from_millis(100));

        let mut past_lines = String::new();
        let mut actual_line: Vec<u8> = Vec::new();
        'main: loop {
            let this_stdout = match communicate.read() {
                Ok((this_stdout_maybe, _)) => this_stdout_maybe.unwrap(),
                Err(err) => match err.kind() {
                    io::ErrorKind::TimedOut => err.capture.0.unwrap(),
                    _ => panic!(err.error),
                },
            };

            for chara in this_stdout {
                if chara == b'\n' {
                    past_lines.push('\n');
                    let decoded_actual_line = String::from_utf8_lossy(&actual_line);
                    if self.redirect {
                        println!("{}", decoded_actual_line);
                    }
                    if decoded_actual_line == self.exit_hash {
                        break 'main
                    }
                    past_lines.push_str(&decoded_actual_line);
                    actual_line = Vec::new();
                } else {
                    actual_line.push(chara);
                }
            };

            if let Some(exit_code) = process.poll() {
                match exit_code {
                    ExitStatus::Exited(0) => break,
                    exit_code => {
                        self.process = None;
                        return Err(KodiCallError::NonZeroExit(past_lines, exit_code));
                    }
                }
            }
        }

        return Ok(past_lines)
    }
}

impl Drop for KodiInterface {
    fn drop(&mut self) {
        if let Some(process) = &mut self.process {
            let _ = process.kill();
        }
    }
}

#[derive(Debug)]
/// represent a kodi/xbmc instance. Each [`Kodi`] instance have a configuration file associated,
/// where kodi store various data, including plugin.
///
/// It also have a cache to store recent results. It can be called by multiple threads.
///
/// It also store a list of unused python thread to not have to start them every time. This is disabled by default, but can be enabled by [`Kodi::set_keep_alive`]. Please note that this may lead to security issue if a plugin is hacked, and a sandbox is put into place for child process.
pub struct Kodi {
    kodi_config_path: String,
    cache: Mutex<TimedCache<PathAccessData, KodiResult>>,
    cached_kodi_interface: Option<Mutex<Vec<KodiInterface>>>,
    python_command: String,
    cache_time: u64,
    cache_size: usize,
    catch_io: bool,
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
            cached_kodi_interface: None,
            python_command: "python2".into(),
            cache_time,
            cache_size,
            catch_io: false,
        }
    }

    /// set the command this program will use to call python. Common value should include ``python2`` and ``python3`` (default to ``python2`` until kodi 19)
    pub fn set_python_command(&mut self, command: String) {
        self.python_command = command;
    }

    /// if ``keep_alive`` is ``true``, it will reuse python thread. If ``false``, it will invoke python every time it need to run a python plugin.
    pub fn set_keep_alive(&mut self, keep_alive: bool) {
        if keep_alive {
            self.cached_kodi_interface = Some(Mutex::new(Vec::new()));
        } else {
            self.cached_kodi_interface = None;
        };
    }

    /// set to ``false`` to display stdout of called python, or ``true`` to not display them.
    pub fn set_catch_io(&mut self, catch_io: bool) {
        self.catch_io = catch_io;
    }

    fn get_commands(&self, tempory_file: &str, access: &PathAccessData) -> Vec<String> {
        let mut result = vec![
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

    fn create_interface(&self) -> KodiInterface {
        KodiInterface::new_with_python(self.python_command.clone(), !self.catch_io)
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

        let mut kodi_interface =
            if let Some(cacked_kodi_interface_mutex) = &self.cached_kodi_interface {
                match cacked_kodi_interface_mutex.lock() {
                    Ok(mut cached_kodi_interface) => match cached_kodi_interface.pop() {
                        Some(kodi_interface) => kodi_interface,
                        None => self.create_interface(),
                    },
                    Err(err) => {
                        error!("the cached kodi interface is poisoned: {:?}", err);
                        self.create_interface()
                    }
                }
            } else {
                self.create_interface()
            };

        //TODO: make this use the sandbox
        let tempory_folder = match tempdir() {
            Ok(value) => value,
            Err(err) => return Err(KodiError::CantCreateTemporyDir(err)),
        };

        let mut data_file: PathBuf = tempory_folder.path().into(); // don't use into_path() to don't persist it
        data_file.push("tmp.json");

        kodi_interface
            .run(
                &self.get_commands(&data_file.to_string_lossy(), &access)
            )
            .map_err(|x| {
                x
            })?;

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

        self.cached_kodi_interface.as_ref().map(|x| {
            match x.lock() {
                Ok(mut cached_kodi_interface) => cached_kodi_interface.push(kodi_interface),
                Err(err) => error!("the cached kodi interface is poisoned: {:?}", err),
            };
        });

        Ok(result)
    }
}
