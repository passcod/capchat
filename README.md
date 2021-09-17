# CAP-CHAT

Sends CAP (Weather, etc) Warnings to Chat.

## Options

|Option|Description|Default|
|:-----|:----------|------:|
|`--cap`|URL for the Atom/RSS feed to CAP alerts (can have multiple)|**required**|
|`--format`|Type of output to send to chatrooms (`json`, `text`, `text+map`).|`text+map`|
|`--severity`|Minimum severity to get alerts for|Minor|
|`--boundaries`|Path to a folder container GeoJSON files with polygons that demarcate areas you care about|`_boundaries` folder in workdir|
|`--outlines`|Path to a folder container GeoJSON files with polygons for outlines of countries or areas, to render basemaps|`_outlines` folder in workdir|
|`--cache-db`|Path to the cache database (used to avoid double-posting)|`_cache` folder in workdir|

You can download outline GeoJSON files from https://geojson-maps.ash.ms/.

You can make your own boundary GeoJSONs with https://geoman.io/geojson-editor.

## Outputs

|Option|Description|
|:-----|:----------|
|`--print`|Print text to STDOUT.|
|`--file`|Write to file. The message will go to `PATH.txt`, and if there's an image it will go to `PATH.png`.|
|`--image-height`|Maximum height of image in pixels for `text+map` output format (default 512).|
|`--image-width`|Maximum width of image in pixels for `text+map` output format (default 512).|
|`--facebook-token`|Facebook Messenger/Workplace token (must have _Message Any Member_ and _Group Chat Bot_ permissions).||
|`--facebook-thread`|Facebook Messenger/Workplace Thread ID to post in. Cannot be a single user chat.||

## Logs

By default, moderate (info) logs are printed to STDERR.
To increase verbosity, use `-v`, `-vv`, or `-vvv`.
To decrease verbosity, use `-q`.

Additionally, and overriding any of these options, the `RUST_LOG` environment variable is respected.
See [tracing-subscriber](https://docs.rs/tracing-subscriber/*/tracing_subscriber/filter/struct.EnvFilter.html) for syntax.

## Example

> ![northland map](./example.png)
>
> SEVERE THUNDERSTORM WATCH
>
> 🟡  **[Auckland,Waikato]**  6 hours from _04:30am Thursday_ to _11:00am Thursday_
>
> Updated at 4:30pm to extend risk of severe thunderstorms to all of northern Waikato
>
> Thunderstorms are expected to develop over the Auckland region this afternoon, and spread southwards into northern Waikato towards evening.
> These thunderstorms will be accompanied by localised heavy rain and small hail.  There is also a low risk of a small tornado or funnel cloud.
> Between 4pm and 10pm today (Thursday) in Auckland, and between 6pm and 11pm in northern Waikato, a few of these thunderstorms could be severe producing localised downpours of 25-40mm/hr.
> Rainfall of this intensity can cause surface and/or flash flooding, especially about low-lying areas such as streams, rivers or narrow valleys, and may also lead to slips.
> Driving conditions will also be hazardous with surface flooding and poor visibility in heavy rain.

## CAP list

### Aotearoa (NZ)

- Weather: https://alerts.metservice.com/cap/rss
- Earthquake: https://api.geonet.org.nz/cap/1.2/GPA1.0/feed/atom1.0/quake
- Civil Defence: https://alerthub.civildefence.govt.nz/rss/pwp
