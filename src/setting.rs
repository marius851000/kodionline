use crate::UserConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Setting {
    pub plugins_to_show: Vec<(String, String)>, //label, path
    pub kodi_path: String,
    pub python_command: String,
    pub default_user_config: UserConfig,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            plugins_to_show: Vec::new(),
            kodi_path: "~/.kodi".into(),
            python_command: "python2".into(), //NOTE: change to python3 once kodi 19 is publicly released
            default_user_config: UserConfig::default(),
        }
    }
}

impl Setting {
    pub fn get_label_for_path(&self, path: &str) -> Option<String> {
        for (label, analyzed_path) in self.plugins_to_show.iter() {
            if path == analyzed_path {
                return Some(label.clone());
            };
        }
        None
    }
}
