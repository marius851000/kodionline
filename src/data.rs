use crate::format_to_string;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Page {
    pub sub_content: Vec<SubContent>,
    //pub sort_methods: Vec<u32>,
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
    pub label: Option<String>,
    pub path: Option<String>,
    pub arts: HashMap<String, Option<String>>,
    pub category: Option<String>,
    pub info: Info,
    pub subtitles: Vec<String>,
    pub properties: HashMap<String, String>,
}

impl ListItem {
    #[must_use]
    pub fn get_display_html(&self) -> String {
        if let Some(value) = &self.label {
            return format_to_string(value);
        };
        "unnamed".to_string()
    }

    pub fn extend(&mut self, other: Self) {
        if self.label.is_none() {
            self.label = other.label.clone();
        };
        if self.path.is_none() {
            self.path = other.path.clone();
        };
        self.arts.extend(other.arts);
        self.info.extend(other.info);
        self.subtitles.extend(other.subtitles);
        self.subtitles.dedup();
        self.properties.extend(other.properties);
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Info {
    #[serde(default)]
    plot: Option<String>,
    #[serde(default)]
    genre: Option<String>,
    #[serde(default)]
    season: Option<u64>,
    #[serde(default)]
    episode: Option<u64>,
    #[serde(default)]
    mediatype: Option<String>,
}

impl Info {
    pub fn extend(&mut self, other: Self) {
        if self.plot.is_none() {
            self.plot = other.plot;
        };
        if self.genre.is_none() {
            self.genre = other.genre;
        };
        if self.season.is_none() {
            self.season = other.season;
        };
        if self.episode.is_none() {
            self.episode = other.episode;
        };
        if self.mediatype.is_none() {
            self.mediatype = other.mediatype;
        };
    }
}
