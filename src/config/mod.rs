use crate::util::Util;

use self::error::{ConfigError, ConfigErrorKind};
use failure::ResultExt;
use log::warn;
use std::collections::HashMap;
use tokio::fs;
use tokio::io;
use tokio::io::AsyncBufReadExt;
use std::path::PathBuf;

#[macro_use]
mod macros;
mod error;

pub type ConfigCollection = HashMap<String, GrabberConfig>;

#[derive(Clone)]
pub struct Replace {
    pub to_replace: String,
    pub replace_with: String,
}

#[derive(Clone)]
pub struct GrabberConfig {
    pub xpath_title: Vec<String>,
    pub xpath_author: Vec<String>,
    pub xpath_date: Vec<String>,
    pub xpath_body: Vec<String>,
    pub xpath_strip: Vec<String>,
    pub strip_id_or_class: Vec<String>,
    pub strip_image_src: Vec<String>,
    pub replace: Vec<Replace>,
    pub single_page_link: Option<String>,
    pub next_page_link: Option<String>,
}

impl GrabberConfig {
    pub async fn parse_directory(directory: &PathBuf) -> Result<ConfigCollection, ConfigError> {
        // create data dir if it doesn't already exist
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(&directory)
            .context(ConfigErrorKind::IO)?;

        let mut dir = tokio::fs::read_dir(directory).await.context(ConfigErrorKind::IO)?;
        let mut collection = HashMap::new();

        while let Ok(entry) = dir.next_entry().await {
            if let Some(entry) = entry {
                if Util::check_extension(&entry, "txt") {
                    if let Ok(config) = GrabberConfig::new(entry.path()).await {
                        let file_name = entry.file_name().to_string_lossy().into_owned();
                        collection.insert(file_name, config);
                    }
                }
            }
        }

        Ok(collection)
    }

    

    async fn new(config_path: PathBuf) -> Result<GrabberConfig, ConfigError> {
        let mut file = fs::File::open(&config_path).await.context(ConfigErrorKind::IO)?;
        let buffer = io::BufReader::new(&mut file);

        let mut xpath_title: Vec<String> = Vec::new();
        let mut xpath_author: Vec<String> = Vec::new();
        let mut xpath_date: Vec<String> = Vec::new();
        let mut xpath_body: Vec<String> = Vec::new();
        let mut xpath_strip: Vec<String> = Vec::new();
        let mut strip_id_or_class: Vec<String> = Vec::new();
        let mut strip_image_src: Vec<String> = Vec::new();
        let mut replace_vec: Vec<Replace> = Vec::new();
        let mut next_page_link: Option<String> = None;
        let mut single_page_link: Option<String> = None;

        // ignore: tidy, prune, autodetect_on_failure and test_url
        let title = "title:";
        let body = "body:";
        let date = "date:";
        let author = "author:";
        let strip = "strip:";
        let strip_id = "strip_id_or_class:";
        let strip_img = "strip_image_src:";
        let single_page = "single_page_link:";
        let next_page = "next_page_link:";
        let find = "find_string:";
        let replace = "replace_string:";
        let replace_single = "replace_string(";

        // ignore these
        let tidy = "tidy:";
        let prune = "prune:";
        let test_url = "test_url:";
        let autodetect = "autodetect_on_failure:";

        let mut lines = buffer.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();
            if line.starts_with('#')
                || line.starts_with(tidy)
                || line.starts_with(prune)
                || line.starts_with(test_url)
                || line.starts_with(autodetect)
                || line.is_empty()
            {
                continue;
            }

            extract_vec_multi!(line, title, xpath_title);
            extract_vec_multi!(line, body, xpath_body);
            extract_vec_multi!(line, date, xpath_date);
            extract_vec_multi!(line, author, xpath_author);

            extract_vec_single!(line, strip, xpath_strip);
            extract_vec_single!(line, strip_id, strip_id_or_class);
            extract_vec_single!(line, strip_img, strip_image_src);

            extract_option_single!(line, single_page, single_page_link);
            extract_option_single!(line, next_page, next_page_link);

            if line.starts_with(replace_single) {
                let value = Util::extract_value(replace_single, line);
                let value: Vec<&str> = value.split("): ").map(|s| s.trim()).collect();
                if value.len() != 2 {
                    continue;
                }

                if let Some(to_replace) = value.get(0) {
                    if let Some(replace_with) = value.get(1) {
                        replace_vec.push(Replace {
                            to_replace: (*to_replace).to_string(),
                            replace_with: (*replace_with).to_string(),
                        });
                    }
                }

                continue;
            }

            if line.starts_with(find) {
                let to_replace = Util::extract_value(find, line).into();

                if let Ok(Some(ref next_line)) = lines.next_line().await {
                    let replace_with = Util::extract_value(replace, &next_line).into();

                    replace_vec.push(Replace {
                        to_replace,
                        replace_with,
                    });
                }
                
                continue;
            }
        }

        if xpath_body.is_empty() {
            warn!("No body xpath found for {}", config_path.display());
            return Err(ConfigErrorKind::BadConfig.into());
        }

        let config = GrabberConfig {
            xpath_title,
            xpath_author,
            xpath_date,
            xpath_body,
            xpath_strip,
            strip_id_or_class,
            strip_image_src,
            replace: replace_vec,
            single_page_link,
            next_page_link,
        };

        Ok(config)
    }
}
