use crate::kodi_recurse_par;
use crate::{AppArgument, RecurseOption, RecurseReport};
use reqwest::{blocking::ClientBuilder, StatusCode};
use std::fs::File;
use std::sync::Arc;

pub fn do_check(
    _app_argument: AppArgument,
    check_argument: AppArgument,
    option: RecurseOption,
) -> Vec<RecurseReport> {
    //TODO: more control on verbosity
    let check_media = check_argument.is_present("check-media");
    let client = Arc::new(ClientBuilder::new().referer(false).build().unwrap());
    kodi_recurse_par::<(), _, _, _>(
        option,
        (),
        move |info, _| {
            let page = info.get_page();
            if let Some(resolved_listitem) = &page.resolved_listitem {
                if check_media {
                    // check if the resolved media exist
                    //TODO: check other referenced content, and make look help look exactly what is wrong
                    if let Some(media_url) = &resolved_listitem.path {
                        if media_url.starts_with("http://") | media_url.starts_with("https://") {
                            let resp = client.clone().get(media_url).send().unwrap();
                            match resp.status() {
                                StatusCode::OK => (),
                                err_code => info.add_error_string(format!(
                                    "getting the distant media at {:?} returned the error code {}",
                                    media_url, err_code
                                )),
                            };
                        }
                        if media_url.starts_with('/') {
                            if let Err(err) = File::open(media_url) {
                                info.add_error_string(format!(
                                    "can't get the local media at {:?}: {:?}",
                                    media_url, err
                                ));
                            };
                        } else {
                            info.add_error_string(format!(
                                "can't determine how to check the existance of {:?}",
                                media_url
                            ));
                        }
                    };
                };
            };
            // check that the IsPlayable flag is valid
            if page.resolved_listitem.is_some() {
                if let Some(sub_content_from_parent) = info.sub_content_from_parent {
                    if !sub_content_from_parent.listitem.is_playable() {
                        info.add_error_string("the data is not marked as playable by one of it parent, but it contain a resolved listitem".to_string());
                    };
                };
            } else {
                if let Some(sub_content_from_parent) = info.sub_content_from_parent {
                    if sub_content_from_parent.listitem.is_playable() {
                        info.add_error_string("the data is marked as playable by one of it parent, but doesn't contain a resolved listitem".to_string());
                    };
                };
            };
            Some(())
        },
        |_, _| false,
        |_, _| (),
    )
}
