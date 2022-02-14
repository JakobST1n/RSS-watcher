# RSS-watcher
Simple rust app that periodically checks RSS feeds for new entries,
and pushes those to Gotify.

## Usage
This can be run using docker or locally, to run with docker you can
```
$ run -it --rm -e DB_HOST=<database host> -e DB_USER=<database user> \
      -e DB_PASS=<database password> -e DB_BASE=<database name> \
      --restart=unless-stopped jakobst1n/rss-watcher
```
To run locally you need to set all those environment variables, and then
you can run it with
```
$ RUST_LOG=info cargo run
```

All feed have to be defined in the database, you should start the app and let
it create the table(s) itself. Then you can add feeds like this
```sql
INSERT INTO `rss-watcher-feeds` (url, push_url, push_token)
     VALUES (<the url of the RSS/Atom feed>,
             <root url of gotify server e.g. https://push.example.com>,
             <token for gotify app>);
```
You can also specify what fields should be used in the title and message fields
of the gotify notification by changing the `title` and `message` columns.
By default they are set to `{{title}}` and `{{summary}}` respectively.

Also, if you set the env var `FETCH_INTERVAL`, it will change how often it 
will poll for new changes (in ms).

## Todo
- Extract more RSS fields.
- Deal with multiple links.
