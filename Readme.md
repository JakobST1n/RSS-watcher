# RSS-watcher
Simple rust app that periodically checks RSS feeds for new entries,
and pushes those to Gotify.

## Requirements
- MySQL database, with a database and authentication prepared that the app can
  use.
- Either you need to have rust and cargo installed, or you need docker.

## Usage
### Docker
The simplest way to run this is using the docker image on 
[docker hub](https://hub.docker.com/r/jakobst1n/rss-watcher) (personally I am
running it on a kubernetes cluster). It can be run with the command below,
make sure to set the database credentials so they fit your database.
```
$ run -it --rm -e DB_HOST=<database host> -e DB_USER=<database user> \
      -e DB_PASS=<database password> -e DB_BASE=<database name> \
      --restart=unless-stopped jakobst1n/rss-watcher
```

### Locally
If you want to run it without docker:
- Make sure Rust and Cargo is installed. 
- Set the environment variables (there are a lot of ways to do this, 
  `export VAR_NAME=VAR_VALUE`, set them before the command, make a small shell
  script to start it, etc...)
- Compile and run the app:
```
$ RUST_LOG=info cargo run
```

### First start
When you start the app the first time, it will create a table in the database,
later it will run migrations between versions automatically. 
If that ever happens.

When the table is created, you can start to add the
feeds you want notifications for. The app starts each iteration by checking
the database. So you can insert new feeds like this in the simplest form:
```sql
INSERT INTO `rss-watcher-feeds` (url, push_url, push_token)
     VALUES (<the url of the RSS/Atom feed>,
             <root url of gotify server e.g. https://push.example.com>,
             <token for gotify app>);
```

## Configuration
### Feeds
The feed config in the database is quite simple, you can however overwrite 
how the feed will be sent to gotify by adjusting the `title` and `message`
fields in the database. By default `title` is set to
`{{title}}: {{entry.title}}` and `message` is set to `{{entry.summary}}`.

The possible template fields are:
| Field                  |
|------------------------|
| {{id}}                 |
| {{title}}              |
| {{updated}}            | 
| {{authors}}            |
| {{description}}        |
| {{links}}              |
| {{categories}}         |
| {{contributors}}       |
| {{language}}           |
| {{published}}          |
| {{rights}}             |
| {{entry.id}}           |
| {{entry.title}}        |
| {{entry.updated}}      |
| {{entry.authors}}      |
| {{entry.links}}        |
| {{entry.summary}}      |
| {{entry.categories}}   |
| {{entry.contributors}} |
| {{entry.published}}    |
| {{entry.source}}       |
| {{entry.rights}}       |

The best way to find the ones you want is to test a bit, here are some resources
to see what they are:
- [https://validator.w3.org/feed/docs/rss2.html](https://validator.w3.org/feed/docs/rss2.html)
- [https://validator.w3.org/feed/docs/atom.html](https://validator.w3.org/feed/docs/atom.html)
- [https://docs.rs/feed-rs/1.0.0/feed_rs/model/struct.Feed.html](https://docs.rs/feed-rs/1.0.0/feed_rs/model/struct.Feed.html)
- [https://docs.rs/feed-rs/1.0.0/feed_rs/model/struct.Entry.html](https://docs.rs/feed-rs/1.0.0/feed_rs/model/struct.Entry.html)

### Environment variables
| Variable       | Description                                                           |
|----------------|-----------------------------------------------------------------------|
| FETCH_INTERVAL | How often the app should poll for new changes in ms (defaults to 2 m) |
| DB_HOST        | Hostname/FQDN/IP address of the database                              |
| DB_BASE        | The database we should use                                            |
| DB_USER        | The user that will be used to access the database                     |
| DB_PASS        | The password that will be used to access the database                 |
| RUST_LOG       | Log level, for docker this defaults to `info`                         |


## Issues
Please make an issue if you find a bug, or if something is weird :)
