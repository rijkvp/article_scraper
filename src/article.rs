use std;
use url::Url;
use std::path::PathBuf;
use chrono::NaiveDateTime;
use crate::error::{
    ScraperError,
    ScraperErrorKind,
};
use std::io::Write;
use failure::ResultExt;

pub struct Article {
    pub title: Option<String>,
    pub author: Option<String>,
    pub url: Url,
    pub date: Option<NaiveDateTime>,
    pub html: Option<String>,
}

impl Article {
    pub fn save_html(&self, path: &PathBuf) -> Result<(), ScraperError> {

        if let Some(ref html) = self.html {
            if let Ok(()) = std::fs::create_dir_all(&path) {
                let mut file_name = match self.title.clone() {
                    Some(file_name) => file_name,
                    None => "Unknown Title".to_owned(),
                };
                file_name.push_str(".html");
                let path = path.join(file_name);
                let mut html_file = std::fs::File::create(&path).context(ScraperErrorKind::IO)?;
                html_file.write_all(html.as_bytes()).context(ScraperErrorKind::IO)?;
            }
        }

        Err(ScraperErrorKind::Unknown)?
    }

}