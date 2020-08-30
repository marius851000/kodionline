use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub struct UserConfig {
    pub language_order: Vec<String>,
    pub resolution_order: Vec<String>,
    pub format_order: Vec<String>
}

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
            source_value.split(":").map(|v| v.to_string()).collect()
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

    /// Add the config in ``other``, treating it as more important. Config from ``other`` will take priority over this one, thus config from this [`UserConfig`] will only be used if the new prioritary config doesn't provide handling for it.
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
    /// // it is undefined if double element are cleaned
    /// ```
    pub fn add_config_prioritary(self, prio: Self) -> Self {
        let mut result = prio;
        result.language_order.extend(self.language_order);
        result.resolution_order.extend(self.resolution_order);
        result.format_order.extend(self.format_order);
        result
    }
}
