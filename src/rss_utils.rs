use crate::database::FeedConf;

use log::{debug, info};
use std::error::Error;
use feed_rs::parser;
use feed_rs::model;
use chrono::prelude::{Utc,DateTime};
use html2md;
extern crate mime;

/**
 * Extract text field from Option
 */
fn extract_text(text: &Option<model::Text>, field: &str) -> String {
    if text.is_none() { return String::from(format!("Field {:#?} was not in feed", field)); }
    let field = text.as_ref().unwrap();
    match (field.content_type.type_(), field.content_type.subtype()) {
        (mime::TEXT, mime::HTML) => return html2md::parse_html(field.content.as_ref()),
        (mime::TEXT, mime::PLAIN) => return field.content.to_owned(),
        _ => return String::from(format!("Unknown field content type {:#?}", field.content_type)),
    }
}

/**
 * Extract string field from Option
 */
fn extract_string(text: &Option<String>, field: &str) -> String {
    if text.is_none() { return String::from(format!("Field {:#?} was not in feed", field)); }
    return text.as_ref().unwrap().to_owned();
}

/**
 * Extract string field from Option
 */
fn extract_datetime(date: &Option<DateTime<Utc>>, field: &str) -> String {
    if date.is_none() { return String::from(format!("Field {:#?} was not in feed", field)); }
    return date.unwrap().to_rfc2822().replace("+0000", "UTC");
}

/**
 * Turn a vector of feed_rs::model::Person into markdown.
 */
fn person_vec_to_md(person_vec: &Vec<model::Person>) -> String {
    let mut md_str = "".to_owned();

    for (i, person) in person_vec.iter().enumerate() {
        if person.uri.is_some() && person.email.is_some() {
            md_str.push_str(format!("[{}]({}) - [homepage]({})", 
                                    person.name, 
                                    person.email.as_ref().unwrap(), 
                                    person.uri.as_ref().unwrap()
                                    ).as_str());
        } else if person.uri.is_some() {
            md_str.push_str(format!("[{}]({})", 
                                    person.name, 
                                    person.uri.as_ref().unwrap(), 
                                    ).as_str());
        } else if person.email.is_some() {
            md_str.push_str(format!("[{}]({})", 
                                    person.name, 
                                    person.email.as_ref().unwrap(), 
                                    ).as_str());
        } else {
            md_str.push_str(&person.name);
        }
        if i < (person_vec.len() - 1) { md_str.push_str(", "); }
    }
    return md_str;
}

/**
 * Turn a vector of feed_rs::model::Link into markdown.
 */
fn link_vec_to_md(link_vec: &Vec<model::Link>) -> String {
    let mut md_str = "".to_owned();

    for (i, link) in link_vec.iter().enumerate() {
        if link.title.is_some() {
            md_str.push_str(format!("[{}]({})", 
                                    &link.title.as_ref().unwrap(), 
                                    &link.href).as_str());
        } else if link.rel.is_some() {
            md_str.push_str(format!("[{}]({})", 
                                    &link.rel.as_ref().unwrap(), 
                                    &link.href).as_str());
        } else {
            md_str.push_str(format!("[{}]({})",
                                    &link.href, 
                                    &link.href).as_str());
        }
        if i < (link_vec.len() - 1) { md_str.push_str(", "); }
    }
    return md_str;
}

/**
 * Turn a vector of feed_rs::model::Category into markdown.
 */
fn category_vec_to_md(category_vec: &Vec<model::Category>) -> String {
    let mut md_str = "".to_owned();

    for (i, category) in category_vec.iter().enumerate() {
        if category.label.is_some() {
            md_str.push_str(category.label.as_ref().unwrap());
        } else {
            md_str.push_str(&category.term);
        }
        if i < (category_vec.len() - 1) { md_str.push_str(", "); }
    }
    return md_str;
}

/**
 * This will replace a given field with the appropriate formatted string from
 * the rss feed/entry/item.
 */
fn fill_template_field(field: &str, entry: &model::Entry, feed: &model::Feed) -> String {
    match field {
        "id" => return feed.id.to_owned(),
        "title" => return extract_text(&feed.title, field).to_owned(),
        "updated" => return extract_datetime(&feed.updated, field).to_owned(),
        "authors" => return person_vec_to_md(&feed.authors).to_owned(),
        "description" => return extract_text(&feed.description, field).to_owned(),
        "links" => return link_vec_to_md(&feed.links).to_owned(),
        "categories" => return category_vec_to_md(&feed.categories).to_owned(),
        "contributors" => return person_vec_to_md(&feed.contributors).to_owned(),
        "language" => return extract_string(&feed.language, field).to_owned(),
        "published" => return extract_datetime(&feed.published, field).to_owned(),
        "rights" => return extract_text(&feed.rights, field).to_owned(),

        "entry.id" => return entry.id.to_owned(),
        "entry.title" => return extract_text(&entry.title, field).to_owned(),
        "entry.updated" => return extract_datetime(&entry.updated, field).to_owned(),
        "entry.authors" => return person_vec_to_md(&entry.authors).to_owned(),
        "entry.links" => return link_vec_to_md(&entry.links).to_owned(),
        "entry.summary" => return extract_text(&entry.summary, field).to_owned(),
        "entry.categories" => return category_vec_to_md(&entry.categories).to_owned(),
        "entry.contributors" => return person_vec_to_md(&entry.contributors).to_owned(),
        "entry.published" => return extract_datetime(&entry.published, field).to_owned(),
        "entry.source" => return extract_string(&entry.source, field).to_owned(),
        "entry.rights" => return extract_text(&entry.rights, field).to_owned(),
        _ => return String::from(format!("Unknown field {:#?}", field))
    }
}

/**
 * Method that escapes some characters that would break json spec, and also escape
 * special HTML characters.
 */
pub fn escape(input: String) -> String {
    return input.replace("\\","\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("&", "$amp;");
}

/**
 * This will find fields in the template string and use fill_template_field
 * to replace the tags with formatted text from the rss feed/entry/item.
 * It does use the escape function on the string it returns.
 */
pub fn fill_template(template_str: &str, entry: &model::Entry, feed: &model::Feed) -> String {
    let mut filled_str = "".to_owned();

    let mut l_bracket_n = 0;
    let mut r_bracket_n = 0;
    let mut field = "".to_owned();

    for c in template_str.chars() {
        if l_bracket_n > 1 {
            if c == '}' {
                r_bracket_n += 1;
                if r_bracket_n > 1 {
                    filled_str.push_str(fill_template_field(&field, 
                                                            &entry, 
                                                            &feed).as_str());
                    field = "".to_owned();
                    r_bracket_n = 0;
                    l_bracket_n = 0;
                }
            } else {
                field.push(c);
            }
        } else if c == '{' {
            l_bracket_n += 1;
            if l_bracket_n > 1 { field = "".to_owned(); }
        } else {
            l_bracket_n = 0;
            filled_str.push(c);
        }
    }
    return escape(filled_str).to_owned();
}

/**
 * Function takes a FeedConf struct, and makes a get request to fetch
 * the feed. It then uses feed_rs to parse that feed and returns that 
 * parsed feed.
 */
pub async fn fetch_feed(feed_conf: &FeedConf, last_fetch_time: DateTime<Utc>) -> Result<Option<model::Feed>, Box<dyn Error>> {
    info!("Fetching feed \"{}\"", &feed_conf.url);
    let client = reqwest::Client::new();
    let last_fetch_rfc2822 = last_fetch_time.to_rfc2822().replace("+0000", "GMT");
    debug!("Using header \"If-Modified-Since {:?}\"", &last_fetch_rfc2822);
    let resp = client.get(&feed_conf.url)
                     .header("If-Modified-Since", &last_fetch_rfc2822)
                     .send()
                     .await?;
    if resp.status() == 304 {
        info!("No changes since last fetch at {}", &last_fetch_rfc2822);
        Ok(None)
    } else {
        let feed = parser::parse(&resp.bytes().await?[..])?;
        debug!("{:#?}", feed);
        Ok(Some(feed))
    }
}
