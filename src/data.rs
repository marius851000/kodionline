use crate::format_to_string;

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
    pub subtitles: Vec<String>,
    pub properties: HashMap<String, String>,
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
            return format_to_string(value);
        };
        "unnamed".to_string()
    }

    /// return ``true`` if this [`LisItem`] is marked as playable
    ///
    /// # Example
    ///
    /// ```
    /// use kodionline::data::ListItem;
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
        if self.album.is_none() {
            self.album = other.album;
        };
        if self.count.is_none() {
            self.count = other.count;
        };
        if self.title.is_none() {
            self.title = other.title;
        };
        if self.artist.is_none() {
            self.artist = other.artist;
        };
        if self.comment.is_none() {
            self.comment = other.comment;
        };
        if self.year.is_none() {
            self.year = other.year;
        };
        if self.duration.is_none() {
            self.duration = other.duration;
        };
    }
}
