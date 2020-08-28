use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Page {
    pub sub_content: Vec<SubContent>,
    pub sort_methods: Vec<u32>,
    pub resolved_listitem: Option<ListItem>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SubContent {
    pub url: String,
    pub is_folder: bool,
    pub total_items: u32,
    pub listitem: ListItem,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ListItem {
    pub label: String,
    pub path: Option<String>,
    pub arts: HashMap<String, Option<String>>,
    //TODO: info, category, properties
    pub info: Info,
    pub subtitles: Vec<String>,
    pub properties: HashMap<String, String>,
}

impl ListItem {
    pub fn get_display_text(&self) -> String {
        self.label.clone()
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Info {
    #[serde(default)]
    plot: Option<String>,
    #[serde(default)]
    genre: Option<String>,
    #[serde(default)]
    season: u64,
    #[serde(default)]
    episode: u64,
    #[serde(default)]
    mediatype: String,
}
