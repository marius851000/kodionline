use crate::{data::SubContent, input::encode_input, should_serve_file, PathAccessData};

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

pub fn get_media_link_subcontent(content: &SubContent, parent: &PathAccessData) -> String {
    let prefix = "/get_media?".to_string();

    if let Some(media_true_url) = &content.listitem.path {
        get_data_link_resolved_url(&media_true_url, &content.url, Vec::new(), prefix, parent)
    } else {
        get_served_data_url(&content.url, Vec::new(), prefix, parent)
    }
}

pub fn get_media_link_resolved_url(
    media_url: &str,
    media_path: &str,
    input: Vec<String>,
    parent: &PathAccessData,
) -> String {
    let prefix = "/get_media?".to_string();
    get_data_link_resolved_url(media_url, media_path, input, prefix, parent)
}

pub fn get_art_link_subcontent(
    content: &SubContent,
    category: &str,
    parent: &PathAccessData,
) -> String {
    let prefix = format!("/get_art?category={}&", category);

    if let Some(Some(art_true_url)) = &content.listitem.arts.get(category) {
        get_data_link_resolved_url(art_true_url, &content.url, Vec::new(), prefix, parent)
    } else {
        get_served_data_url(&content.url, Vec::new(), prefix, parent)
    }
}

pub fn get_data_link_resolved_url(
    media_url: &str,
    media_path: &str,
    input: Vec<String>,
    prefix: String,
    parent: &PathAccessData,
) -> String {
    if should_serve_file(media_url) {
        get_served_data_url(media_path, input, prefix, parent)
    } else {
        media_url.to_string()
    }
}

pub fn get_served_data_url(
    path: &str,
    input: Vec<String>,
    prefix: String,
    parent: &PathAccessData,
) -> String {
    format!(
        "{}path={}&input={}&parent_path={}&parent_input={}",
        prefix,
        utf8_percent_encode(path, NON_ALPHANUMERIC),
        encode_input(&input),
        utf8_percent_encode(&parent.path, NON_ALPHANUMERIC),
        encode_input(&parent.input)
    )
}
