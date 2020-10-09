use crate::{kodi_recurse_par, AppArgument, RecurseOption, RecurseReport, ReportBuilder};
use kodi_rust::data::{ListItem, SubContent};
use reqwest::{blocking::ClientBuilder, StatusCode};
use serde::Serialize;
use serde_json;
use std::fs;
use std::fs::DirBuilder;
use std::fs::File;
use std::io::ErrorKind;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize)]
#[serde(tag = "type")]
enum SavePossibility {
    Directory(SaveDirectory),
    Media(SaveMedia),
}

#[derive(Serialize)]
struct SaveDirectory {
    sub_content: Vec<(String, SubContent)>,
}

#[derive(Serialize)]
struct SaveMedia {
    media_file_name: String,
    media_url: String,
    listitem: ListItem,
}

#[derive(Clone)]
struct ParentInfo {
    parent_path: PathBuf,
    is_top_level: bool,
}

fn get_extension(media_path: &str) -> Option<String> {
    let last_splited_by_dot = media_path.split(".").last()?;
    if last_splited_by_dot.len() > 5 {
        None
    } else {
        Some(format!(".{}", last_splited_by_dot.to_string()))
    }
}

fn get_label_from_listitem(list: &ListItem) -> Option<String> {
    list.label.clone()
}

//TODO:
fn encode_path(path: String) -> String {
    path
}

fn fetch_media(save_path: PathBuf, media_url: &str) -> Result<(), ReportBuilder> {
    let client = ClientBuilder::new().referer(false).build().unwrap();
    if media_url.starts_with("http://") | media_url.starts_with("https://") {
        let resp = client.get(media_url).send().unwrap();
        match resp.status() {
            StatusCode::OK => {
                let bytes = match resp.bytes() {
                    Ok(v) => v,
                    Err(err) => {
                        return Err(ReportBuilder::new_error(format!(
                            "can't download the file at {}",
                            media_url
                        ))
                        .add_tip(format!("the error returned by resp.bytes() is {:?}", err)))
                    }
                }; //TODO: get rid of unwrap
                let mut save_file = match File::create(&save_path) {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(ReportBuilder::new_error(format!(
                            "can't open the file at {} in write mode",
                            save_path.to_string_lossy()
                        ))
                        .add_tip(format!("the error returned by File::create is {:?}", err)))
                    }
                };
                save_file.write(&bytes).unwrap(); //TODO: get rid of unwrap
            }
            err_code => {
                return Err(ReportBuilder::new_error(format!(
                    "getting the distant media at {:?} returned the error code {}",
                    media_url, err_code
                )))
            }
        }
    } else {
        //local file
        if let Err(err) = fs::copy(media_url, &save_path) {
            return Err(ReportBuilder::new_error(format!(
                "can't copy the file from {} to {}",
                media_url,
                save_path.to_string_lossy()
            )));
        };
    }
    Ok(())
}

fn get_child_dir(
    parent_data: &ParentInfo,
    sub_content_from_parent: Option<&SubContent>,
) -> Result<PathBuf, ReportBuilder> {
    let mut this_dir = parent_data.parent_path.clone();
    if !parent_data.is_top_level {
        if let Some(sub_content) = sub_content_from_parent {
            let this_subfolder_name: String =
                encode_path(match get_label_from_listitem(&sub_content.listitem) {
                    Some(value) => value,
                    None => {
                        return Err(ReportBuilder::new_error(
                            "can't find a label for this element, so can't save to a folder"
                                .to_string(),
                        )
                        .add_tip(format!("listitem is : {:?}", sub_content.listitem)))
                    }
                });
            this_dir.push(this_subfolder_name);
        } else {
            return Err(ReportBuilder::new_error("can't get the sub content from the parent, even thought this isn't declared as a children".to_string())
                .set_internal_error(true));
        }
    }
    Ok(this_dir)
}
pub fn get_success_path(mut folder: PathBuf) -> PathBuf {
    folder.push(".success");
    return folder;
}

pub fn do_mirror(
    app_argument: AppArgument,
    mirror_argument: AppArgument,
    option: RecurseOption,
) -> Vec<RecurseReport> {
    kodi_recurse_par::<ParentInfo, _, _, _>(
        option,
        ParentInfo {
            parent_path: PathBuf::from(mirror_argument.value_of("dest-path").unwrap()),
            is_top_level: true,
        },
        move |info, parent_data| {
            // already checked we should mirror this path
            // step 0: parse data about the file

            let to_save = if let Some(resolved_listitem) = &info.get_page().resolved_listitem {
                if !info.get_page().sub_content.is_empty() {
                    info.add_report(ReportBuilder::new_error(
                        "the folder have both a resolved listitem and sub folder !".to_string(),
                    ));
                    return None;
                };

                let media_url = match &resolved_listitem.path {
                    Some(v) => v.to_string(),
                    None => {
                        info.add_report(ReportBuilder::new_error(
                            "can't find the path for the this media".to_string(),
                        ));
                        return None;
                    }
                };

                SavePossibility::Media(SaveMedia {
                    media_file_name: format!(
                        "media{}",
                        if let Some(extension) = get_extension(&media_url) {
                            extension
                        } else {
                            "".into()
                        }
                    ),
                    media_url,
                    listitem: resolved_listitem.clone(),
                })
            } else {
                //empty folder are totally fine
                let mut child_sub_contents = Vec::new();
                for sub_cont in &info.get_page().sub_content {
                    child_sub_contents.push(match get_label_from_listitem(&sub_cont.listitem) {
                        Some(v) => (encode_path(v), sub_cont.clone()),
                        None => {
                            info.add_report(
                                ReportBuilder::new_error(
                                    "can't find a label for a child".to_string(),
                                )
                                .add_tip(format!("child's listitem : {:?}", sub_cont.listitem)),
                            );
                            return None;
                        }
                    });
                }

                SavePossibility::Directory(SaveDirectory {
                    sub_content: child_sub_contents,
                })
            };
            // step 1: find the destination folder
            let this_dir = get_child_dir(&parent_data, info.get_sub_content_from_parent()).unwrap(); //TODO: get rid of unwrap

            // step 2: write the data
            match DirBuilder::new().create(this_dir.clone()) {
                Ok(()) => (),
                Err(err) => match err.kind() {
                    ErrorKind::AlreadyExists => (),
                    _ => {
                        info.add_report(
                            ReportBuilder::new_error(format!(
                                "can't create the folder for mirroring at {:?}",
                                this_dir
                            ))
                            .set_internal_error(true)
                            .add_tip(format!("error of DirBuilder::new : {:?}", err)),
                        );
                        return None;
                    }
                },
            };

            let mut this_data_path = this_dir.clone();
            this_data_path.push("data.json");
            let mut data_file = match File::create(&this_data_path) {
                Ok(v) => v,
                Err(err) => {
                    info.add_report(
                        ReportBuilder::new_error(format!("can't create the file at {:?} (supposed to contain data for a mirrored path)", this_data_path))
                            .set_internal_error(true)
                            .add_tip(format!("error of File::Create : {:?}", err))
                    );
                    return None;
                }
            };

            match serde_json::to_writer_pretty(&mut data_file, &to_save) {
                Ok(()) => (),
                Err(err) => {
                    info.add_report(
                        ReportBuilder::new_error(format!("can't export the data to a json file"))
                            .set_internal_error(true)
                            .add_tip(format!("the path in question is {:?}", this_data_path))
                            .add_tip(format!("error of serde_json::to_writer_pretty : {:?}", err)),
                    );
                    return None;
                }
            };
            // step 3: fetch the media
            if let SavePossibility::Media(media_data) = to_save {
                let mut media_path = this_dir.clone();
                media_path.push(media_data.media_file_name);
                match fetch_media(media_path, &media_data.media_url) {
                    Ok(()) => (),
                    Err(mut report_error) => {
                        info.add_report(
                            report_error.add_tip(
                                "happened while downloading the main media file".to_string(),
                            ),
                        );
                        return None;
                    }
                };
            };

            Some(ParentInfo {
                parent_path: this_dir,
                is_top_level: false,
            })
        },
        |info, parent_data| {
            let child_path =
                get_child_dir(&parent_data, info.get_sub_content_from_parent()).unwrap(); //TODO: get rid of unwrap
            let success_path = get_success_path(child_path);
            success_path.exists()
        },
        |info, parent_data| {
            let child_path =
                get_child_dir(&parent_data, info.get_sub_content_from_parent()).unwrap(); //TODO: get rid of unwrap
            let success_path = get_success_path(child_path);
            File::create(success_path).unwrap();
        },
    )
}
