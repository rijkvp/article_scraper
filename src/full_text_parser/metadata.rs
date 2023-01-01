use chrono::{DateTime, Utc};
use libxml::xpath::Context;
use log::{debug, warn};
use std::str::FromStr;
use crate::{article::Article, util::Util};
use super::config::ConfigEntry;

pub fn extract(
    context: &Context,
    config: Option<&ConfigEntry>,
    global_config: &ConfigEntry,
    article: &mut Article,
) {
    
    if article.title.is_none() {
        article.title = extract_title(context, config, global_config).and_then(|title| Some(match escaper::decode_html(&title) {
            Ok(escaped_title) => escaped_title,
            Err(_error) => title,
        }));
    }

    if article.author.is_none() {
        article.author = extract_author(context, config, global_config).and_then(|author| Some(match escaper::decode_html(&author) {
            Ok(escaped_author) => escaped_author,
            Err(_error) => author,
        }));
    }

    if article.date.is_none() {
        article.date = extract_date(context, config, global_config);
    }
}

fn extract_title(
    context: &Context,
    config: Option<&ConfigEntry>,
    global_config: &ConfigEntry
) -> Option<String> {
    // check site specific config
    if let Some(config) = config {
        for xpath_title in &config.xpath_title {
            if let Ok(title) = Util::extract_value_merge(context, xpath_title) {
                debug!("Article title: '{}'", title);
                return Some(title);
            }
        }
    }

    // check global config
    for xpath_title in &global_config.xpath_title {
        if let Ok(title) = Util::extract_value_merge(context, xpath_title) {
            debug!("Article title: '{}'", title);
            return Some(title);
        }
    }

    // generic meta (readablity)
    get_meta(context, "dc:title")
        .or_else(|| get_meta(context, "dcterm:title"))
        .or_else(|| get_meta(context, "og:title"))
        .or_else(|| get_meta(context, "weibo:article:title"))
        .or_else(|| get_meta(context, "weibo:webpage:title"))
        .or_else(|| get_meta(context, "title"))
        .or_else(|| get_meta(context, "twitter:title"))
}

fn extract_author(
    context: &Context,
    config: Option<&ConfigEntry>,
    global_config: &ConfigEntry
) -> Option<String> {
    // check site specific config
    if let Some(config) = config {
        for xpath_author in &config.xpath_author {
            if let Ok(author) = Util::extract_value(context, xpath_author) {
                debug!("Article author: '{}'", author);
                return Some(author);
            }
        }
    }

    // check global config
    for xpath_author in &global_config.xpath_author {
        if let Ok(author) = Util::extract_value(context, xpath_author) {
            debug!("Article author: '{}'", author);
            return Some(author);
        }
    }

    // generic meta (readablity)
    get_meta(context, "dc:creator")
        .or_else(|| get_meta(context, "dcterm:creator"))
        .or_else(|| get_meta(context, "author"))
}

fn extract_date(
    context: &Context,
    config: Option<&ConfigEntry>,
    global_config: &ConfigEntry
) -> Option<DateTime<Utc>> {
    // check site specific config
    if let Some(config) = config {
        for xpath_date in &config.xpath_date {
            if let Ok(date_string) = Util::extract_value(context, xpath_date) {
                debug!("Article date: '{}'", date_string);
                if let Ok(date) = DateTime::from_str(&date_string) {
                    return Some(date);
                } else {
                    warn!("Parsing the date string '{}' failed", date_string);
                }
            }
        }
    }

    // check global config
    for xpath_date in &global_config.xpath_date {
        if let Ok(date_string) = Util::extract_value(context, xpath_date) {
            debug!("Article date: '{}'", date_string);
            if let Ok(date) = DateTime::from_str(&date_string) {
                return Some(date);
            } else {
                warn!("Parsing the date string '{}' failed", date_string);
            }
        }
    }

    None
}

fn get_meta(context: &Context, name: &str) -> Option<String> {
    Util::get_attribute(context, &format!("//meta[contains(@name, '{}')]", name), "content").ok()
}