use std::process;
use std::env;
use log::{debug, info, warn, error};
use mysql::*;
use mysql::prelude::*;

#[derive(Debug, PartialEq, Eq)]
pub struct FeedConf {
    pub id: u32,
    pub url: String,
    pub last_fetch: Option<i64>,
    pub title: String,
    pub message: String,
    pub push_url: String,
    pub push_token: String
}

/**
 * Create Opts struct from env vars.
 */
fn build_opts() -> Opts {
    let db_host = env::var("DB_HOST").expect("$DB_HOST is not set");
    let db_base = env::var("DB_BASE").expect("$DB_BASE is not set");
    let db_user = env::var("DB_USER").expect("$DB_USER is not set");
    let db_pass = env::var("DB_PASS").expect("$DB_PASS is not set");
    return Opts::from(OptsBuilder::new()
            .ip_or_hostname(Some(db_host))
            .user(Some(db_user))
            .pass(Some(db_pass))
            .db_name(Some(db_base)));
}

pub fn new_conn() -> Option<Conn> {
    let conn_res = Conn::new(build_opts());
    if let Err(ref x) = conn_res {
        error!("Could not connect to database ({:#?})...", x);
        return None;
    }
    return Some(conn_res.unwrap());
}

/**
 * Check wether the table `rss-watcher-feeds` exists.
 */
fn table_exists(conn: &mut Conn) -> bool {
    let db_base = env::var("DB_BASE").expect("$DB_BASE is not set");
    let q = conn.prep("SELECT table_name \
                         FROM INFORMATION_SCHEMA.TABLES \
                        WHERE TABLE_SCHEMA=:schema \
                              AND TABLE_NAME='rss-watcher-feeds'").unwrap();
    let res: Option<String> = conn.exec_first(
        &q, params! {"schema" => db_base}).unwrap();
    if let None = res { return false; }
    return true;
}

/**
 * This will create the `rss-watcher-feeds` table.
 */
fn table_create(conn: &mut Conn) {
    let db_base = env::var("DB_BASE").expect("$DB_BASE is not set");
    info!("Creating table `{}`.`rss-watcher-feeds`", db_base);
    let mut tx = conn.start_transaction(TxOpts::default()).unwrap();
    let mut q = "CREATE TABLE `rss-watcher-feeds` ( \
                      `id` int NOT NULL AUTO_INCREMENT, \
                      `url` VARCHAR(255) NOT NULL, \
                      `last_fetch` int, \
                      `title` VARCHAR(255) NOT NULL DEFAULT '{{title}}', \
                      `message` VARCHAR(255) NOT NULL DEFAULT '{{summary}}', \
                      `push_url` VARCHAR(255) NOT NULL, \
                      `push_token` VARCHAR(255) NOT NULL, \
                      PRIMARY KEY (`id`)
                 )";
    if let Err(x) = tx.query_drop(q) {
        error!("Could not create table! ({:#?}", x);
        process::exit(1);
    }
    q = "INSERT INTO `rss-watcher-feeds` (id,
                                          url,
                                          last_fetch,
                                          title,
                                          message,
                                          push_url,
                                          push_token)
                                   VALUES (0,'version',1,'','','','')";
    if let Err(x) = tx.query_drop(q) {
        error!("Could not insert versioning row! ({:#?}", x);
        process::exit(1);
    }
    if let Err(x) = tx.commit() {
        error!("Could not create table! ({:#?}", x);
        process::exit(1);
    }

}

/**
 * Select the row in the table describing the database version.
 */
fn get_db_version(conn: &mut Conn) -> i64 {
    let q = "SELECT `last_fetch` from `rss-watcher-feeds` WHERE `id`=0 AND `url` LIKE 'version'";
    let res: Result<Option<i64>> = conn.query_first(q);
    if let Err(x) = res {
        error!("Could not get current version from database ({:#?})...", x);
        process::exit(1);
    }
    let res_res = res.unwrap();
    if let None = res_res {
        error!("Row with id=0 and url='version' does not exist, something is wrong!");
        error!("Please fix your database manually!");
        process::exit(1);
    }
    return res_res.unwrap();
}

/**
 * Run migrations v2.
 */
fn run_migrations_v2(tx: &mut Transaction, version: i64) {
    if version < 2 {
        warn!("Running migrations to v2");
        let mut q;
        q = "ALTER TABLE `rss_watcher`.`rss-watcher-feeds` \
             CHANGE COLUMN `title` `title` VARCHAR(255) NOT NULL DEFAULT '{{title}}: {{entry.title}}' , \
             CHANGE COLUMN `message` `message` VARCHAR(255) NOT NULL DEFAULT '{{entry.summary}}';";

        if let Err(x) = tx.query_drop(q) {
            error!("Could not run database migration to v2...! ({:#?}", x);
            process::exit(1);
        }

        q = "UPDATE `rss-watcher-feeds` SET `last_fetch`=2 WHERE `id`=0";
        if let Err(x) = tx.query_drop(q) {
            error!("Could not run database migration to v2...! ({:#?}", x);
            process::exit(1);
        }
    }
}

/**
 * Bootstrap the database, this will make sure tables exists,
 * create them if not and run migrations if nececarry.
 */
pub fn bootstrap() {
    info!("Bootstrapping database");
    let conn_res = Conn::new(build_opts());
    if let Err(ref x) = conn_res {
        error!("Could not connect to database ({:#?})...", x);
        process::exit(1);
    }
    let mut conn = conn_res.unwrap();
    info!("Connected to database");
    
    if !table_exists(&mut conn) {
        table_create(&mut conn);
    }

    let version = get_db_version(&mut conn);
    if version < 2 {
        let res_tx = conn.start_transaction(TxOpts::default());
        if let Err(x) = res_tx {
            error!("Could not create transaction for updating last fetch time! {:#?}", x);
            return;
        }
        let mut tx = res_tx.unwrap();

        run_migrations_v2(&mut tx, version);

        if let Err(x) = tx.commit() {
            warn!("Could not commit update! ({:#?}", x);
        }
    } else {
        info!("Database is up to date, no migrations to run.");
    }

    info!("Database should now be bootstrapped");
    info!("We are assuming that the table has the correct columns");
    info!("If not, we are going to get sql errors");
}

/**
 * This will fetch all feeds from the database and return them as a Vector.
 */
pub fn get_feeds(conn: &mut Conn) -> Option<Vec<FeedConf>> {
    let q = "SELECT `id`, \
                    `url`, \
                    `last_fetch`, \
                    `title`, \
                    `message`, \
                    `push_url`, \
                    `push_token` \
               FROM `rss-watcher-feeds` \
              WHERE id > 0";
    let res = conn.query_map(q,
               |(id,url,last_fetch,title,message,push_url,push_token)| {
                 FeedConf{id,url,last_fetch,title,message,push_url,push_token}
               },);
    debug!("{:#?}", res);
    match res {
        Ok(r) => return Some(r),
        Err(e) => {
            error!("Could not get feeds from database ({:?})", e);
            return None;
        }
    }
}

/**
 * Method that updates the last fetch time timestamp in the database
 */
pub fn update_last_fetch(feed_id: u32, last_fetch: i64, conn: &mut Conn) {
    let res_tx = conn.start_transaction(TxOpts::default());
    if let Err(x) = res_tx {
        error!("Could not create transaction for updating last fetch time! {:#?}", x);
        return;
    }
    let mut tx = res_tx.unwrap();

    let q = "UPDATE `rss-watcher-feeds` SET last_fetch=?  WHERE id=?";
    if let Err(x) = tx.exec_drop(q, (last_fetch,feed_id,)) {
        warn!("Could not update last fetch time...! ({:#?}", x);
    }
    if let Err(x) = tx.commit() {
        warn!("Could not commit update! ({:#?}", x);
    }

}
