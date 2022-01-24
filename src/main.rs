mod database;
use database::FeedConf;

use std::env;
use std::process;
use std::error::Error;
use feed_rs::parser;
use feed_rs::model::Feed;
use feed_rs::model::Entry;
use feed_rs::model::Text;
use chrono::prelude::{Utc,DateTime,NaiveDateTime};
use std::time::Duration;
use tokio::{time};
use log::{info, warn, error};
use html2md;
extern crate mime;

/**
 * Extract text field from Option
 */
fn extract_text(text: &Option<Text>) -> String {
    if text.is_none() { return String::from("Text field not found"); }
    let field = text.as_ref().unwrap();
    match (field.content_type.type_(), field.content_type.subtype()) {
        (mime::TEXT, mime::HTML) => return html2md::parse_html(field.content.as_ref()),
        (mime::TEXT, mime::PLAIN) => return field.content.to_owned(),
        _ => return String::from(format!("Unknown field content type {:#?}", field.content_type)),
    }
}

/**
 * This will extract fields from RSS entry, and replace special tags
 * from the input string with those entries.
 */
fn replace_tags(input: String, entry: &Entry) -> String {
    let mut out = input;
    out = out.replace("{{id}}", entry.id.as_ref());
    out = out.replace("{{title}}", extract_text(&entry.title).as_ref());
    out = out.replace("{{summary}}", extract_text(&entry.summary).as_ref());
    return out;
}

/**
 * Method that escapes some characters that would break json spec
 */
fn escape(input: String) -> String {
    return input.replace("\\","\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n");
}

/**
 * Push feed entry to gotify
 */
async fn gotify_push(entry: &Entry, feed_conf: &FeedConf) -> Result<(), reqwest::Error>  {
    let uri = format!("{}/message", &feed_conf.push_url);

    // Extract content and create title and message strings
    let mut title_content = feed_conf.title.to_owned();
    let mut message_content = feed_conf.message.to_owned();
    title_content = replace_tags(title_content, entry);
    message_content = replace_tags(message_content, entry);

    // Build json string that will be sent as payload to gotify
    let mut req = "{".to_owned();

    req.push_str(format!("\"title\":\"{}\"", escape(title_content.to_owned())).as_str());
    req.push_str(format!(",\"message\":\"{}\"", escape(message_content.to_owned())).as_str());
    req.push_str(",\"priority\":1");
    
    req.push_str(",\"extras\": {");
    req.push_str("\"client::display\": { \"contentType\": \"text/markdown\" }");
    if entry.links.len() > 0 {
        req.push_str(",\"client::notification\": { \"click\": { \"url\": \"");
        req.push_str(escape(entry.links[0].href.to_owned()).as_str());
        req.push_str("\"}}")
    }
    req.push_str("}}");

    // Send request to gotify
    let client = reqwest::Client::new();
    let res = client.post(uri)
                    .query(&[("token",&feed_conf.push_token)])
                    .body(req.to_owned())
                    .header("Content-Type", "application/json")
                    .send()
                    .await?;
    if res.status().is_success() {
        info!("Sent notification with title \"{}\"", title_content);
    } else {
        error!("payload: {}", req);
        error!("Could not send notification... {:#?}", res);
    }
    Ok(())
}

/**
 * Function takes a FeedConf struct, and makes a get request to fetch
 * the feed. It then uses feed_rs to parse that feed and returns that 
 * parsed feed.
 */
async fn fetch_feed(feed_conf: &FeedConf, last_fetch_time: DateTime<Utc>) -> Result<Option<Feed>, Box<dyn Error>> {
    info!("Fetching feed \"{}\"", &feed_conf.url);
    let client = reqwest::Client::new();
    let last_fetch_rfc2822 = last_fetch_time.to_rfc2822().replace("+0000", "GMT");
    let resp = client.get(&feed_conf.url)
                     .header("If-Modified-Since", &last_fetch_rfc2822)
                     .send()
                     .await?;
    if resp.status() == 304 {
        info!("No changes since last fetch {}", &last_fetch_rfc2822);
        Ok(None)
    } else {
        let feed = parser::parse(&resp.bytes().await?[..])?;
        Ok(Some(feed))
    }
}

/**
 * This calls fetch_feed, and figures out wether it succeeded or not.
 * It then pushes all _new_ entries to gotify, and returns the last fetched
 * time (now, or current if there is no new articles).
 */
async fn get_feed(feed_conf: &FeedConf) -> bool {
    // Check wether last_fetch_time is set, if it is not, we will use the "now"
    // time as that. Which means that no articles will be found.
    let last_fetch_time;
    match &feed_conf.last_fetch {
        Some(x) => {
            last_fetch_time = DateTime::from_utc(
                NaiveDateTime::from_timestamp(x.to_owned(),0), Utc);
            error!("{:#?}", last_fetch_time);
        },
        None => last_fetch_time = Utc::now(),
    }

    // Fetch the feed and parse it
    let res = fetch_feed(&feed_conf, last_fetch_time).await;
    let feed: Option<Feed>;
    match res {
        Err(e) =>  {
            error!("Could not fetch feed ({:?})", e);
            return false;
        },
        Ok(x) => feed = x
    }

    // If feed is empty (we got status code 304), we should skip any further 
    // processing
    if let None = feed { return false; }

    // Process all entries in the feed
    for entry in feed.unwrap().entries {
        // Skip sending notification if the publish time is before the
        // last_fetch_time
        if let Some(x) = entry.published {
            if last_fetch_time > x {
                info!("Skipping entry that was published at {}", x);
                continue;
            }
        }
        // Attempt to send notification, give up feed for this main loop
        // iteration without saving last_fetch_time
        if let Err(e) = gotify_push(&entry, &feed_conf).await {
            error!("Could not send push notification ({:#?})", e);
            return false;
        }
    }

    return true;
}

/**
 * This gets all feeds from the database and fetches them once.
 */
async fn main_loop() {
    let mut conn = database::new_conn();
    for feed in database::get_feeds(&mut conn) {
        let time_now = Utc::now();
        let res = get_feed(&feed).await;
        if res {
            database::update_last_fetch(feed.id, time_now.timestamp(), &mut conn);
        }
    }
}

/** 
 * Main app, sets up database, and then it keeps an active loop.
 */
async fn app() {
    database::bootstrap();

    let interval_timeout;
    match env::var("FETCH_INTERVAL") {
        Ok(val) => {
            let res = val.parse::<u64>();
            if let Err(_e) = res {
                error!("Invalid $FETCH_INTERVAL value {:#?}", val);
                process::exit(1);
            }
            interval_timeout = res.unwrap();
        },
        Err(_e) => {
            warn!("$FETCH_INTERVAL not set, using default of 2m");
            interval_timeout = 120000;
        },
    }

    let mut interval = time::interval(Duration::from_millis(interval_timeout));
    loop {
        main_loop().await;
        interval.tick().await;
    }
}

fn main() {
    env_logger::init();
    info!("Starting rss-watcher");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let future = app();
    rt.block_on(future);
}
