use kodi_rust::{
    data::{KodiResult, ListItem},
    encode_url, get_sub_content_from_parent, should_serve_file, Kodi, PathAccessData, Setting,
    UserConfig,
};

use log::{error, info};
use rocket::{
    http::RawStr, response, response::NamedFile, response::Redirect, response::Responder, Request,
    State,
};

pub enum ServeDataFromPlugin {
    Redirect(Redirect),
    NamedFile(NamedFile),
}

impl<'r> Responder<'r> for ServeDataFromPlugin {
    fn respond_to(self, request: &Request) -> response::Result<'r> {
        match self {
            Self::Redirect(r) => r.respond_to(request),
            Self::NamedFile(f) => f.respond_to(request),
        }
    }
}

pub fn redirect_data_generic<F>(
    kodi: State<Kodi>,
    access: PathAccessData,
    parent_access_option: Option<PathAccessData>,
    category_label: &str,
    get_path_function: F,
) -> Option<ServeDataFromPlugin>
where
    F: Fn(&ListItem) -> Option<String>,
{
    let create_result_for_url = |data_url: String| -> Option<ServeDataFromPlugin> {
        println!("found {:?}", data_url);
        if should_serve_file(&data_url) {
            //TODO: check if the file is permitted to be read
            Some(ServeDataFromPlugin::NamedFile(
                match NamedFile::open(data_url) {
                    Ok(file) => file,
                    Err(err) => {
                        error!("failed to open the local file due to {:?}", err);
                        return None;
                    }
                },
            ))
        } else {
            let encoded = encode_url(&data_url);
            info!(
                "redirecting the {} at {:?} to \"{}\"",
                category_label, access, encoded
            );
            Some(ServeDataFromPlugin::Redirect(Redirect::to(encoded)))
        }
    };

    // try the parent first, as it probably already in the cache
    if let Some(parent_access) = parent_access_option {
        if let Some(sub_content_from_parent) =
            get_sub_content_from_parent(&kodi, &parent_access, &access.path)
        {
            if let Some(data_url) = get_path_function(&sub_content_from_parent.listitem) {
                if !data_url.starts_with("plugin://") {
                    return create_result_for_url(data_url);
                }
            }
        }
    };

    // otherwise, try to get it from the child
    match kodi.invoke_sandbox(&access) {
        Ok(KodiResult::Content(page)) => match page.resolved_listitem {
            Some(resolved_listitem) => match get_path_function(&resolved_listitem) {
                Some(media_url) => create_result_for_url(media_url),
                None => {
                    error!(
                        "can't find the searched {} for {:?}",
                        category_label, resolved_listitem
                    );
                    None
                }
            },
            None => {
                error!("can't find the resolved listitem for {:?}", access);
                None
            }
        },
        Ok(result) => {
            error!(
                "asked for input to access {} at {:?} (result: {:?})",
                category_label, access, result
            );
            None
        }
        Err(err) => {
            error!(
                "error {:?} while serving {} at {:?}",
                err, category_label, access
            );
            None
        }
    }
}

#[get("/get_media?<path>&<input>&<parent_path>&<parent_input>&<c>")]
pub fn redirect_media(
    kodi: State<Kodi>,
    setting: State<Setting>,
    path: String,
    input: Option<&RawStr>,
    parent_path: Option<String>,
    parent_input: Option<&RawStr>,
    c: Option<String>,
) -> Option<ServeDataFromPlugin> {
    let config_in_url = UserConfig::new_from_optional_uri(c);
    let final_config = setting
        .default_user_config
        .clone()
        .add_config_prioritary(config_in_url);

    redirect_data_generic(
        kodi,
        PathAccessData::new(path, input.map(|x| x.as_str()), final_config.clone()),
        PathAccessData::try_create_from_url(parent_path, parent_input.map(|x| x.as_str()), final_config),
        "media",
        |x| x.path.clone(),
    )
}

#[get("/get_art?<category>&<path>&<input>&<parent_path>&<parent_input>&<c>")]
#[allow(clippy::too_many_arguments)]
pub fn redirect_art(
    kodi: State<Kodi>,
    setting: State<Setting>,
    category: String,
    path: String,
    input: Option<&RawStr>,
    parent_path: Option<String>,
    parent_input: Option<&RawStr>,
    c: Option<String>,
) -> Option<ServeDataFromPlugin> {
    let config_in_url = UserConfig::new_from_optional_uri(c);
    let final_config = setting
        .default_user_config
        .clone()
        .add_config_prioritary(config_in_url);

    redirect_data_generic(
        kodi,
        PathAccessData::new(path, input.map(|x| x.as_str()), final_config.clone()),
        PathAccessData::try_create_from_url(parent_path, parent_input.map(|x| x.as_str()), final_config),
        "art",
        |x| match &x.arts.get(&category) {
            //TODO: this line is anormaly long. Find how to shorten it
            Some(art_url_option) => match *art_url_option {
                Some(value) => Some(value.clone()),
                None => None,
            },
            None => None,
        },
    )
}
