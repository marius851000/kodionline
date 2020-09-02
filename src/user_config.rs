use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

//TODO: type for the pair of V, no_child_V (ensure to have the same Serde parsing way)

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Hash, Eq)]
pub struct UserConfig {
    #[serde(default)]
    pub language_order: Vec<String>,
    #[serde(default)]
    pub no_child_language_order: bool,
    #[serde(default)]
    pub resolution_order: Vec<String>,
    #[serde(default)]
    pub no_child_resolution_order: bool,
    #[serde(default)]
    pub format_order: Vec<String>,
    #[serde(default)]
    pub no_child_format_order: bool,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            language_order: vec!["en".into()],
            no_child_language_order: false,
            resolution_order: vec!["720p".into(), "480p".into(), "360p".into(), "1080p".into()],
            no_child_resolution_order: false,
            format_order: vec!["mp4".into(), "webm".into(), "ogv".into()],
            no_child_format_order: false,
        }
    }
}

const URISPECIAL: &AsciiSet = &CONTROLS.add(b'%').add(b'!').add(b'.');

impl UserConfig {
    /// Create a new empty [`UserConfig`]
    pub fn new_empty() -> Self {
        Self {
            language_order: Vec::new(),
            no_child_language_order: false,
            resolution_order: Vec::new(),
            no_child_resolution_order: false,
            format_order: Vec::new(),
            no_child_format_order: false,
        }
    }

    /// create a user config based on an [`HashMap`] of [`String`] with a [`String`] keyword
    ///
    /// valid key are (see the [`UserConfig`] documentation for information on their meaning):
    /// - ``lang_ord`` : the order of language. They are seperated using a ``:`` character.
    /// - ``res_ord`` : the order of resolution. They are separated with a ``:``  character.
    /// - ``form_ord`` : the order of format. Also use a ``:`` for separation.
    ///
    /// if ``nc-<key>`` is equal to ``t`` (for ``true``), all the child that will be merged with lower priority with [`UserConfig::add_config_prioritary`] are ignored.
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
    /// config.insert("nc-res_ord".into(), "t".into());
    /// config.insert("nc-inv".into(), "t".into());
    /// config.insert("useless".into(), "none".into());
    ///
    /// let user_config = UserConfig::new_from_dict(config);
    ///
    /// assert_eq!(user_config.language_order, vec!["fr".to_string(), "en".to_string()]);
    /// assert_eq!(user_config.resolution_order, vec!["1080p".to_string()]);
    /// assert_eq!(user_config.no_child_resolution_order, true);
    /// assert_eq!(user_config.format_order, Vec::<String>::new());
    /// ```
    pub fn new_from_dict(mut dict: HashMap<String, String>) -> Self {
        let dict_ref_mut = &mut dict;

        let mut set_double_dot_use_and_drain_if_in_dict = move |result_list: &mut Vec<String>, result_use: &mut bool, keyword: &str| {
            if let Some(list) = dict_ref_mut.remove(keyword) {
                *result_list = split_double_dot(list)
            }
            if let Some(first) = dict_ref_mut.remove(&format!("nc-{}", keyword)) {
                *result_use = &first == "t"
            }
        };

        fn split_double_dot(source_value: String) -> Vec<String> {
            source_value.split(':').map(|v| v.to_string()).collect()
        }

        let mut result = Self::new_empty();

        set_double_dot_use_and_drain_if_in_dict(&mut result.language_order, &mut result.no_child_language_order, "lang_ord");
        set_double_dot_use_and_drain_if_in_dict(&mut result.resolution_order, &mut result.no_child_resolution_order, "res_ord");
        set_double_dot_use_and_drain_if_in_dict(&mut result.format_order, &mut result.no_child_format_order, "form_ord");

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
    /// the URI is encoded under the form: ``key.value!key2.value2``. the key and value are percent decoded after parsing.
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
    /// let mut source = UserConfig::new_empty();
    /// source.language_order = vec!["fr".into(), "!nv.li-=d".into()];
    /// source.resolution_order = vec!["la%li!".into()];
    ///
    /// assert_eq!(
    ///     source,
    ///     UserConfig::new_from_optional_uri(Some("lang_ord.fr:%21nv%2eli-=d!res_ord.la%25li%21".into()))
    /// );
    /// ```
    pub fn new_from_optional_uri(uri: Option<String>) -> Self {
        match uri {
            Some(uri) => {
                let mut result_hashmap = HashMap::new();
                for section in uri.split('!') {
                    let mut splited = section.split('.');
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
            None => Self::new_empty(),
        }
    }

    /// Encode into a [`String`] this configuration
    ///
    /// the string is under the form ``key.value!key2.value2``. `!`, `.`, `%`, controls and utf-8 characters of key and value are url encoded.
    ///
    /// The string can be decoded with [`UserConfig::new_from_optional_uri`].
    ///
    /// # Example
    ///
    /// ```
    /// use kodionline::UserConfig;
    ///
    /// let mut source = UserConfig::new_empty();
    /// source.language_order = vec!["fr".into(), "!nv/li-=d".into()];
    /// source.resolution_order = vec!["la%li!".into()];
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
            result.push('.');
            result.push_str(&utf8_percent_encode(value, URISPECIAL).to_string());
        }
        result
    }

    /// Add the config in ``other``, treating it as more important. Config from ``other`` will take priority over this one, thus config from this [`UserConfig`] will only be used if the new prioritary config doesn't provide handling for it.
    ///
    /// after merging, the element are deduplicated (implemented by a call to [`UserConfig::clean`])
    ///
    /// if the no_child_* are set, then the value of ``Self`` are ignored for this list.
    /// # Example
    ///
    /// ```
    /// use kodionline::UserConfig;
    ///
    /// let mut static_config = UserConfig::new_empty();
    /// static_config.language_order = vec!["fr".into()];
    /// static_config.resolution_order = vec!["1080p".into(), "720p".into()];
    /// static_config.format_order = vec!["mp4".into(), "webm".into()];
    ///
    /// let mut dynamic_config = UserConfig::new_empty();
    /// dynamic_config.language_order = vec!["en".into()];
    /// dynamic_config.resolution_order = vec!["720p".into()];
    /// dynamic_config.format_order = vec!["ogv".into()];
    /// dynamic_config.no_child_format_order = true;
    ///
    /// let result_config = static_config.add_config_prioritary(dynamic_config);
    /// assert_eq!(result_config.language_order, vec!["en".to_string(), "fr".to_string()]);
    /// assert_eq!(&result_config.resolution_order[0], "720p");
    /// assert_eq!(&result_config.resolution_order[1], "1080p");
    /// assert_eq!(result_config.format_order, vec!["ogv".to_string()]);
    /// assert_eq!(result_config.no_child_format_order, false);
    /// ```
    pub fn add_config_prioritary(self, prio: Self) -> Self {
        fn extend_dict_if_not_set<T>(to_extend: &mut Vec<T>, data: Vec<T>, not_extend: &mut bool) {
            if *not_extend {
                *not_extend = false;
            } else {
                to_extend.extend(data);
            }
        }
        let mut result = prio;
        extend_dict_if_not_set(&mut result.language_order, self.language_order, &mut result.no_child_language_order);
        extend_dict_if_not_set(&mut result.resolution_order, self.resolution_order, &mut result.no_child_resolution_order);
        extend_dict_if_not_set(&mut result.format_order, self.format_order, &mut result.no_child_format_order);
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
            no_child_language_order: self.no_child_language_order,
            no_child_resolution_order: self.no_child_resolution_order,
            no_child_format_order: self.no_child_format_order,
        }
    }
}
