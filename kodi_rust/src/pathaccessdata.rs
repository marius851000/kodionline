use crate::{
    escape_tag,
    input::{decode_input, encode_input},
    UserConfig,
};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::Serialize;

/// Store an data required to define the way to acceed to a kodi plugin virtual folder.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct PathAccessData {
    /// the path of the folder, of the kind ``plugin://<plugin id>/<internal path>``
    pub path: String,
    /// the various inputs that will be provided if the plugin ask user input. The first element will be given first.
    pub input: Vec<String>,
    /// the configuration that will be passed along with this url. Plugin may change content they display based on that.
    pub config: UserConfig,
}

impl PathAccessData {
    /// create a new [`PathAccessData`] based on the ``path``, ``input`` (if exist) and ``config``.
    ///
    /// input is decoded with [`decode_input`]. That is, percent encoded string separated by ``:``
    ///
    /// # Example
    ///
    /// ```
    /// use kodi_rust::{PathAccessData, UserConfig};
    /// PathAccessData::new("plugin://.../...".to_string(), Some("a:b"), UserConfig::default());
    /// ```
    pub fn new(path: String, input: Option<&str>, config: UserConfig) -> Self {
        PathAccessData {
            path,
            input: input.map(|x| decode_input(x)).unwrap_or_else(|| Vec::new()),
            config,
        }
    }

    /// Shortcut for [`PathAccessData::new`]. Return ``None`` if path is ``None``, else return [`PathAccessData::new`] with the three arguments.
    pub fn try_create_from_url(
        path: Option<String>,
        input: Option<&str>,
        config: UserConfig,
    ) -> Option<Self> {
        path.map(|x| Self::new(x, input, config))
    }
}

/// Contain a representation of [`PathAccessData`] dedicated to be displayed in a web page.
#[derive(Serialize)]
pub struct PathAccessFormat {
    /// the path, urlencoded
    pub path_safe: String,
    /// the path, html escaped
    pub path_escaped: String,
    /// the input, encoded, and safe to put into an url (all but ``:`` and alphanumeric are absent)
    pub input_encoded: String,
    /// the config
    pub config: UserConfig,
    /// the uri of the config, that may be then decoded. It can be safely embedded into a webpage. May contain alphanumeric, ``!`` and ``.``
    pub config_uri_safe: String,
}

impl PathAccessFormat {
    pub fn new_from_pathaccessdata(path_access_data: PathAccessData) -> Self {
        PathAccessFormat {
            path_safe: utf8_percent_encode(&path_access_data.path, NON_ALPHANUMERIC).to_string(),
            path_escaped: escape_tag(path_access_data.path),
            input_encoded: encode_input(&path_access_data.input),
            config_uri_safe: path_access_data.config.encode_to_uri(),
            config: path_access_data.config,
        }
    }
}
