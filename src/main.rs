mod rss_utils;
mod notify;
mod database;
use database::FeedConf;

use std::env;
use std::process;
use log::{debug, info, warn, error};

use chrono::prelude::{Utc,DateTime,NaiveDateTime};
use tokio::{time};
use std::time::Duration;
use feed_rs::model::Feed;

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
        Some(x) => last_fetch_time = DateTime::from_utc(
                       NaiveDateTime::from_timestamp_opt(x.to_owned(),0).unwrap(), Utc),
        None => last_fetch_time = Utc::now(),
    }
    debug!("Using last_fetch_time {:?}", last_fetch_time.to_owned());

    // Fetch the feed and parse it
    let res = rss_utils::fetch_feed(&feed_conf, last_fetch_time).await;
    let feed_res: Option<Feed>;
    match res {
        Err(e) =>  {
            error!("Could not fetch feed ({:?})", e);
            return false;
        },
        Ok(x) => feed_res = x
    }

    // If feed is empty (we got status code 304), we should skip any further 
    // processing
    if let None = feed_res { return false; }
    let feed = feed_res.unwrap();

    // Process all entries in the feed
    let res_notif = notify::all(&feed, &feed_conf, last_fetch_time).await;
    return res_notif;
}

/**
 * This gets all feeds from the database and fetches them once.
 */
async fn main_loop() {
    info!("========== Checking for new feed entries now");

    let res_conn = database::new_conn();
    if let None = res_conn {
        error!("Could not open database connection, waiting until next iteration before trying again!");
        return;
    };
    let mut conn = res_conn.unwrap();

    let res_feeds = database::get_feeds(&mut conn);
    
    if let None = res_feeds {
        error!("Could not get feeds, waiting until next iteration before trying again!");
        return;
    }

    let feeds = res_feeds.unwrap();
    info!("           Got {} feeds to check", feeds.len());

    for feed in feeds {
        let time_now = Utc::now();
        if get_feed(&feed).await {
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
