use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq, Hash, Eq)]
pub struct UserConfig {
    #[serde(default)]
    pub language_order: Vec<String>,
    #[serde(default)]
    pub resolution_order: Vec<String>,
    #[serde(default)]
    pub format_order: Vec<String>,
}

const URISPECIAL: &AsciiSet = &CONTROLS.add(b'%').add(b'!').add(b'=');

impl UserConfig {
    /// create a user config based on an [`HashMap`] of [`String`] with a [`String`] keyword
    ///
    /// valid key are (see the [`UserConfig`] documentation for information on their meaning):
    /// - ``lang_ord`` : the order of language. They are seperated using a ``:`` character.
    /// - ``res_ord`` : the order of resolution. They are separated with a ``:``  character.
    /// - ``form_ord`` : the order of format. Also use a ``:`` for separation.
    ///
    /// all other keys are silently ignored
    ///
    /// # Example
    ///
    /// ```
    /// use kodionline::UserConfig;
    /// use std::collections::HashMap;
    ///
    /// let mut config = HashMap::new();
    /// config.insert("lang_ord".into(), "fr:en".into());
    /// config.insert("res_ord".into(), "1080p".into());
    /// config.insert("useless".into(), "none".into());
    ///
    /// let user_config = UserConfig::new_from_dict(config);
    ///
    /// assert_eq!(user_config.language_order, vec!["fr".to_string(), "en".to_string()]);
    /// assert_eq!(user_config.resolution_order, vec!["1080p".to_string()]);
    /// assert_eq!(user_config.format_order, Vec::<String>::new());
    /// ```
    pub fn new_from_dict(mut dict: HashMap<String, String>) -> Self {
        fn split_double_dot(source_value: String) -> Vec<String> {
            source_value.split(':').map(|v| v.to_string()).collect()
        }

        let mut result = Self::default();

        if let Some(language_order) = dict.remove("lang_ord") {
            result.language_order = split_double_dot(language_order);
        };

        if let Some(resolution_order) = dict.remove("res_ord") {
            result.resolution_order = split_double_dot(resolution_order);
        };

        if let Some(format_order) = dict.remove("form_ord") {
            result.format_order = split_double_dot(format_order);
        };

        result
    }

    /// tranform this [`UserSetting`] in an [`HashMap`] that can be read by [`UserSetting::new_from_dict`]
    //TODO: test
    pub fn to_dict(&self) -> HashMap<String, String> {
        fn add_double_dot(source: &[String]) -> String {
            let mut result = String::new();
            for (count, value) in source.iter().enumerate() {
                if count > 0 {
                    result.push(':');
                };
                result.push_str(value);
            }
            result
        };

        let mut result = HashMap::new();
        result.insert("lang_ord".into(), add_double_dot(&self.language_order));
        result.insert("res_ord".into(), add_double_dot(&self.resolution_order));
        result.insert("form_ord".into(), add_double_dot(&self.format_order));
        result
    }

    /// Create a new [`UserConfig`] based on the given uri (if existing). Value use the default value if unspecified or the uri is [`None`]
    ///
    /// the URI is encoded under the form: ``key=value!key2=value2``. the key and value are percent decoded after parsing.
    ///
    /// The resulting hashmap is then parsed by [`UserConfig::new_from_dict`].
    ///
    /// if a key is set multiple time, the last one will be used.
    ///
    /// In case of invalid input, the result is undefined. The function will try to set valid input anyway.
    ///
    /// # Example
    ///
    /// ```
    /// use kodionline::UserConfig;
    ///
    /// assert_eq!(
    ///     UserConfig {
    ///         language_order: vec!["fr".into(), "!nv/li-=d".into()],
    ///         resolution_order: vec!["la%li!".into()],
    ///         .. UserConfig::default()
    ///     },
    ///     UserConfig::new_from_optional_uri(Some("lang_ord=fr:%21nv/li-%3dd!res_ord=la%25li%21".into()))
    /// );
    /// ```
    pub fn new_from_optional_uri(uri: Option<String>) -> Self {
        match uri {
            Some(uri) => {
                let mut result_hashmap = HashMap::new();
                for section in uri.split('!') {
                    let mut splited = section.split('=');
                    let key = percent_decode_str(&match splited.next() {
                        Some(v) => v,
                        None => continue,
                    })
                    .decode_utf8_lossy()
                    .to_string();
                    let value = percent_decode_str(&match splited.next() {
                        Some(v) => v,
                        None => continue,
                    })
                    .decode_utf8_lossy()
                    .to_string();
                    result_hashmap.insert(key, value);
                }
                Self::new_from_dict(result_hashmap)
            }
            None => Self::default(),
        }
    }

    /// Encode into a [`String`] this configuration
    ///
    /// the string is under the form ``key=value!key2=value2``. `!`, `=`, `%`, controls and utf-8 characters of key and value are url encoded.
    ///
    /// The string can be decoded with [`UserConfig::new_from_optional_uri`].
    ///
    /// # Example
    ///
    /// ```
    /// use kodionline::UserConfig;
    ///
    /// let source = UserConfig {
    ///     language_order: vec!["fr".into(), "!nv/li-=d".into()],
    ///     resolution_order: vec!["la%li!".into()],
    ///     .. UserConfig::default()
    /// };
    ///
    /// assert_eq!(
    ///     UserConfig::new_from_optional_uri(Some(source.encode_to_uri())),
    ///     source
    /// );
    /// ```
    pub fn encode_to_uri(&self) -> String {
        let to_encode = self.to_dict();
        let mut result = String::new();

        let mut first_element = true;
        for (key, value) in to_encode.iter() {
            if value == "" {
                continue;
            };
            if !first_element {
                result.push('!');
            } else {
                first_element = false;
            }
            result.push_str(&utf8_percent_encode(key, URISPECIAL).to_string());
            result.push('=');
            result.push_str(&utf8_percent_encode(value, URISPECIAL).to_string());
        }
        result
    }

    /// Add the config in ``other``, treating it as more important. Config from ``other`` will take priority over this one, thus config from this [`UserConfig`] will only be used if the new prioritary config doesn't provide handling for it.
    ///
    /// after merging, the element are deduplicated (implemented by a call to [`UserConfig::clean`])
    ///
    /// # Example
    ///
    /// ```
    /// use kodionline::UserConfig;
    ///
    /// let static_config = UserConfig {
    ///     language_order: vec!["fr".into()],
    ///     resolution_order: vec!["1080p".into(), "720p".into()],
    ///     .. UserConfig::default()
    /// };
    ///
    /// let dynamic_config = UserConfig {
    ///     language_order: vec!["en".into()],
    ///     resolution_order: vec!["720p".into()],
    ///      .. UserConfig::default()
    /// };
    ///
    /// let result_config = static_config.add_config_prioritary(dynamic_config);
    /// assert_eq!(result_config.language_order, vec!["en".to_string(), "fr".to_string()]);
    /// assert_eq!(&result_config.resolution_order[0], "720p");
    /// assert_eq!(&result_config.resolution_order[1], "1080p");
    /// ```
    pub fn add_config_prioritary(self, prio: Self) -> Self {
        let mut result = prio;
        result.language_order.extend(self.language_order);
        result.resolution_order.extend(self.resolution_order);
        result.format_order.extend(self.format_order);
        result.clean()
    }

    /// remove duplicated
    pub fn clean(self) -> Self {
        //TODO: search for a library to do this
        fn remove_duplicate(mut list: Vec<String>) -> Vec<String> {
            let mut known_value: HashSet<String> = HashSet::new();
            let mut result = Vec::new();
            for entry in list.drain(..) {
                if known_value.contains(&entry) {
                    continue;
                };
                result.push(entry.clone());
                known_value.insert(entry);
            }
            result
        };

        Self {
            language_order: remove_duplicate(self.language_order),
            resolution_order: remove_duplicate(self.resolution_order),
            format_order: remove_duplicate(self.format_order),
        }
    }
}
