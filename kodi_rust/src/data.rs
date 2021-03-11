use crate::{extend_option, format_to_string};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum KodiResult {
    Content(Page),
    Keyboard(Keyboard),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Keyboard {
    pub default: Option<String>,
    pub heading: Option<String>,
    pub hidden: bool,
}

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

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ListItem {
    pub label: Option<String>,
    pub path: Option<String>,
    pub arts: HashMap<String, Option<String>>,
    pub category: Option<String>,
    pub info: Info,
    pub subtitles: Vec<Option<String>>,
    pub properties: HashMap<String, String>,
    pub x_avalaible_languages: Vec<String>,
    pub stream_info: StreamInfo,
}

impl ListItem {
    #[must_use]
    pub fn get_thumb_category(&self) -> Option<&'static str> {
        for key in &["thumb", "poster"] {
            if let Some(art_url_option) = self.arts.get(*key) {
                if art_url_option.is_some() {
                    return Some(key);
                }
            }
        }
        None
    }

    #[must_use]
    pub fn get_display_html(&self) -> String {
        if let Some(value) = &self.label {
             format_to_string(value)
        } else if let Some(name) = &self.info.title {
            format_to_string(name)
        } else {
            "unnamed".to_string()
        }
    }

    /// return ``true`` if this [`LisItem`] is marked as playable
    ///
    /// # Example
    ///
    /// ```
    /// use kodi_rust::data::ListItem;
    ///
    /// let mut playable_listitem = ListItem::default();
    /// playable_listitem.properties.insert("isPlayable".to_string(), "true".to_string());
    /// assert!(playable_listitem.is_playable());
    ///
    /// let unplayable_listitem = ListItem::default();
    /// assert!(!unplayable_listitem.is_playable());
    /// ```
    #[must_use]
    pub fn is_playable(&self) -> bool {
        for is_playable_key in &["IsPlayable", "isPlayable", "Isplayable", "isplayable"] {
            if let Some(is_playable_value) = self
                .properties
                .get(&is_playable_key.to_string())
                .map(|x| x.as_str())
            {
                return matches!(is_playable_value, "true" | "True" | "TRUE");
            };
        }
        false
    }

    pub fn extend(&mut self, other: Self) {
        extend_option(&mut self.label, other.label);
        extend_option(&mut self.path, other.path);
        self.arts.extend(other.arts);
        self.info.extend(other.info);
        self.subtitles.extend(other.subtitles);
        self.subtitles.dedup();
        self.properties.extend(other.properties);
        self.x_avalaible_languages
            .extend(other.x_avalaible_languages);
        self.x_avalaible_languages.dedup();
        self.stream_info.extend(other.stream_info);
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Info {
    #[serde(default)]
    pub plot: Option<String>,
    #[serde(default)]
    pub season: Option<u64>,
    #[serde(default)]
    pub episode: Option<u64>,
    #[serde(default)]
    pub mediatype: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub count: Option<u64>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub year: Option<i64>,
    #[serde(default)]
    pub duration: Option<u64>,
}

impl Info {
    pub fn extend(&mut self, other: Self) {
        extend_option(&mut self.plot, other.plot);
        extend_option(&mut self.genre, other.genre);
        extend_option(&mut self.season, other.season);
        extend_option(&mut self.episode, other.episode);
        extend_option(&mut self.mediatype, other.mediatype);
        extend_option(&mut self.album, other.album);
        extend_option(&mut self.count, other.count);
        extend_option(&mut self.title, other.title);
        extend_option(&mut self.artist, other.artist);
        extend_option(&mut self.comment, other.comment);
        extend_option(&mut self.year, other.year);
        extend_option(&mut self.duration, other.duration);
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct StreamInfo {
    #[serde(default)]
    pub audio: StreamInfoAudio,
}

impl StreamInfo {
    pub fn extend(&mut self, other: Self) {
        self.audio.extend(other.audio)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct StreamInfoAudio {
    #[serde(default)]
    pub language: Option<String>,
}

impl StreamInfoAudio {
    pub fn extend(&mut self, other: Self) {
        extend_option(&mut self.language, other.language)
    }
}
