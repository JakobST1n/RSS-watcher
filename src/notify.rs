use crate::database::FeedConf;
use crate::rss_utils;

use chrono::prelude::{DateTime, Utc};
use feed_rs::model::Feed;
use log::{error, info};

/**
 * Push feed entry to gotify
 */
async fn gotify(
    title: String,
    message: String,
    link: Option<String>,
    feed_conf: &FeedConf,
) -> Result<(), reqwest::Error> {
    let uri = format!("{}/message", &feed_conf.push_url);

    // Build json string that will be sent as payload to gotify
    let mut req = "{".to_owned();

    req.push_str(format!("\"title\":\"{}\"", title).as_str());
    req.push_str(format!(",\"message\":\"{}\"", message).as_str());
    req.push_str(",\"priority\":1");

    req.push_str(",\"extras\": {");
    req.push_str("\"client::display\": { \"contentType\": \"text/markdown\" }");
    if link.is_some() {
        req.push_str(",\"client::notification\": { \"click\": { \"url\": \"");
        req.push_str(link.unwrap().as_str());
        req.push_str("\"}}")
    }
    req.push_str("}}");

    // Send request to gotify
    let client = reqwest::Client::new();
    let res = client
        .post(uri)
        .query(&[("token", &feed_conf.push_token)])
        .body(req.to_owned())
        .header("Content-Type", "application/json")
        .send()
        .await?;
    if res.status().is_success() {
        info!("Sent notification with title \"{}\"", title);
    } else {
        error!("payload: {}", req);
        error!("Could not send notification... {:#?}", res);
    }
    Ok(())
}

/**
 * Push all new entries in the feed as per the configuration
 */
pub async fn all(feed: &Feed, feed_conf: &FeedConf, last_fetch_time: DateTime<Utc>) -> bool {
    let mut all_notifs_successfull = true;

    // Skip sending notification if the publish time is before the
    // last_fetch_time
    for entry in &feed.entries {
        if let Some(x) = entry.published {
            if last_fetch_time > x {
                info!("Skipping entry that was published at {}", x);
                continue;
            }
        }

        // Get the fields we want to send to gotify
        let title = rss_utils::fill_template(&feed_conf.title, &entry, &feed);
        let message = rss_utils::fill_template(&feed_conf.message, &entry, &feed);
        let mut link: Option<String> = None;
        if entry.links.len() > 0 {
            link = Some(rss_utils::escape(entry.links[0].href.to_owned()));
        }

        if let Err(e) = gotify(title, message, link, &feed_conf).await {
            error!("Could not send push notification ({:#?})", e);
            all_notifs_successfull = false;
        }
    }

    return all_notifs_successfull;
}
