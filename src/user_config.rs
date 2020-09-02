use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(from = "Vec<T>")]
#[serde(into = "Vec<T>")]
pub struct OverridableVec<T: Clone + Eq + Hash> {
    pub value: Vec<T>,
    pub no_child: bool,
}

impl<T: Eq + Clone + Hash> OverridableVec<T> {
    fn add_child_and_reset_no_child(&mut self, child: OverridableVec<T>) {
        if self.no_child {
            self.no_child = false;
        } else {
            self.value.extend(child.value);
        }
    }
}

impl<T: Eq + Clone + Hash> From<Vec<T>> for OverridableVec<T> {
    fn from(value: Vec<T>) -> Self {
        OverridableVec {
            value,
            no_child: false,
        }
    }
}

impl<T: Eq + Clone + Hash> From<OverridableVec<T>> for Vec<T> {
    fn from(overridable: OverridableVec<T>) -> Self {
        overridable.value
    }
}

impl<T: Eq + Clone + Hash> Deref for OverridableVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Eq + Clone + Hash> DerefMut for OverridableVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: Eq + Clone + Hash> PartialEq for OverridableVec<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq + Clone + Hash> Eq for OverridableVec<T> {}

impl<T: Eq + Clone + Hash> Hash for OverridableVec<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Hash, Eq)]
pub struct UserConfig {
    #[serde(default)]
    pub language_order: OverridableVec<String>,
    #[serde(default)]
    pub resolution_order: OverridableVec<String>,
    #[serde(default)]
    pub format_order: OverridableVec<String>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            language_order: vec!["en".into()].into(),
            resolution_order: vec!["720p".into(), "480p".into(), "360p".into(), "1080p".into()]
                .into(),
            format_order: vec!["mp4".into(), "webm".into(), "ogv".into()].into(),
        }
    }
}

impl UserConfig {
    /// Create a new empty [`UserConfig`]
    pub fn new_empty() -> Self {
        Self {
            language_order: OverridableVec::default(),
            resolution_order: OverridableVec::default(),
            format_order: OverridableVec::default(),
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
    /// assert_eq!(*user_config.language_order, vec!["fr".to_string(), "en".to_string()]);
    /// assert_eq!(*user_config.resolution_order, vec!["1080p".to_string()]);
    /// assert_eq!(user_config.resolution_order.no_child, true);
    /// assert_eq!(*user_config.format_order, Vec::<String>::new());
    /// ```
    pub fn new_from_dict(mut dict: HashMap<String, String>) -> Self {
        let dict_ref_mut = &mut dict;

        let mut set_double_dot_use_and_drain_if_in_dict =
            move |result: &mut OverridableVec<String>, keyword: &str| {
                if let Some(list) = dict_ref_mut.remove(keyword) {
                    **result = split_double_dot(list);
                }
                if let Some(first) = dict_ref_mut.remove(&format!("nc-{}", keyword)) {
                    result.no_child = &first == "t";
                }
            };

        fn split_double_dot(source_value: String) -> Vec<String> {
            source_value.split(':').map(|v| v.to_string()).collect()
        }

        let mut result = Self::new_empty();

        set_double_dot_use_and_drain_if_in_dict(&mut result.language_order, "lang_ord");
        set_double_dot_use_and_drain_if_in_dict(&mut result.resolution_order, "res_ord");
        set_double_dot_use_and_drain_if_in_dict(&mut result.format_order, "form_ord");

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
        result.insert("lang_ord".into(), add_double_dot(&*self.language_order));
        result.insert("res_ord".into(), add_double_dot(&*self.resolution_order));
        result.insert("form_ord".into(), add_double_dot(&*self.format_order));
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
    /// *source.language_order = vec!["fr".into(), "!nv.li-=d".into()];
    /// *source.resolution_order = vec!["la%li!".into()];
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
    /// the string is under the form ``key.value!key2.value2``. Non alphanumeric (including utf-8 characters) of key and value are url encoded.
    ///
    /// The string can be decoded with [`UserConfig::new_from_optional_uri`].
    ///
    /// The result can be safely embedded into html, or any other code that doesn't consider ``%`` and alphanumeric character as special character.
    ///
    /// # Example
    ///
    /// ```
    /// use kodionline::UserConfig;
    ///
    /// let mut source = UserConfig::new_empty();
    /// *source.language_order = vec!["fr".into(), "!nv/li-=d".into()];
    /// *source.resolution_order = vec!["la%li!".into()];
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
            result.push_str(&utf8_percent_encode(key, NON_ALPHANUMERIC).to_string());
            result.push('.');
            result.push_str(&utf8_percent_encode(value, NON_ALPHANUMERIC).to_string());
        }
        result
    }

    /// Add the config in ``other``, treating it as more important. Config from ``other`` will take priority over this one, thus config from this [`UserConfig`] will only be used if the new prioritary config doesn't provide handling for it.
    ///
    /// after merging, the element are deduplicated (implemented by a call to [`UserConfig::clean`])
    ///
    /// if the no_child value is set, then the value of ``Self`` are ignored for this list, and no_child is set to false in the returned value.
    ///
    /// # Example
    ///
    /// ```
    /// use kodionline::UserConfig;
    ///
    /// let mut static_config = UserConfig::new_empty();
    /// *static_config.language_order = vec!["fr".into()];
    /// *static_config.resolution_order = vec!["1080p".into(), "720p".into()];
    /// *static_config.format_order = vec!["mp4".into(), "webm".into()];
    ///
    /// let mut dynamic_config = UserConfig::new_empty();
    /// *dynamic_config.language_order = vec!["en".into()];
    /// *dynamic_config.resolution_order = vec!["720p".into()];
    /// *dynamic_config.format_order = vec!["ogv".into()];
    /// dynamic_config.format_order.no_child = true;
    ///
    /// let result_config = static_config.add_config_prioritary(dynamic_config);
    /// assert_eq!(*result_config.language_order, vec!["en".to_string(), "fr".to_string()]);
    /// assert_eq!(&*result_config.resolution_order[0], "720p");
    /// assert_eq!(&*result_config.resolution_order[1], "1080p");
    /// assert_eq!(*result_config.format_order, vec!["ogv".to_string()]);
    /// assert_eq!(result_config.format_order.no_child, false);
    /// ```
    pub fn add_config_prioritary(self, prio: Self) -> Self {
        let mut result = prio;
        result
            .language_order
            .add_child_and_reset_no_child(self.language_order);
        result
            .resolution_order
            .add_child_and_reset_no_child(self.resolution_order);
        result
            .format_order
            .add_child_and_reset_no_child(self.format_order);
        result.clean();
        result
    }

    /// remove duplicated
    pub fn clean(&mut self) {
        //TODO: search for a library to do this
        fn remove_duplicate(list: &mut Vec<String>) {
            let mut new_list = Vec::new();
            let mut known_value: HashSet<String> = HashSet::new();
            for entry in list.drain(..) {
                if known_value.contains(&entry) {
                    continue;
                };
                new_list.push(entry.clone());
                known_value.insert(entry);
            }
            *list = new_list;
        };

        remove_duplicate(&mut *self.language_order);
        remove_duplicate(&mut *self.resolution_order);
        remove_duplicate(&mut *self.format_order);
    }
}
