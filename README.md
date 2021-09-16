# CAP-CHAT

Sends CAP (Weather, etc) Warnings to Chat.

## Options

|Option|Description|Default|
|:-----|:----------|------:|
|`--cap`|URL for the Atom/RSS feed to CAP alerts (can have multiple)|**required**|
|`--output`|Type of output to send to chatrooms (`json`, `text`, `image`, `map`); if `json` chat arguments are ignored and output is only printed to screen.|`map`|
|`--severity`|Minimum severity to get alerts for|Minor|
|`--boundaries`|Path to a folder container GeoJSON files with polygons that demarcate areas you care about|Working directory|
|`--cache-db`|Path to the cache database (used to avoid double-posting)|`_cache` folder in workdir|
|`--fb-workplace-token`|Facebook Workplace token (must have _Message Any Member_ and _Group Chat Bot_ permissions)||
|`--fb-workplace-thread`|Facebook Workplace Thread ID to post in||

Note that if you provide no boundary GeoJSON files, all alerts will be filtered out (making this tool rather useless).

## Logs

By default, moderate (info) logs are printed to STDERR.
To increase verbosity, use `-v`, `-vv`, or `-vvv`.
To decrease verbosity, use `-q`.

Additionally, and overriding any of these options, the `RUST_LOG` environment variable is respected.
See [tracing-subscriber](https://docs.rs/tracing-subscriber/*/tracing_subscriber/filter/struct.EnvFilter.html) for syntax.

## CAP list

### Aotearoa (NZ)

- Weather: https://alerts.metservice.com/cap/rss
- Earthquake: https://api.geonet.org.nz/cap/1.2/GPA1.0/feed/atom1.0/quake
- Civil Defence: https://alerthub.civildefence.govt.nz/rss/pwp
