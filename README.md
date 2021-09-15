# CAP-CHAT

Sends Weather Warnings to Workplace Chat.

## Args

|Arg|Description|Default|
|:--|:----------|------:|
|`--cap-rss URL`|URL for the RSS feed to CAP alerts (can have multiple)|https://alerts.metservice.com/cap/rss|
|`--boundaries PATH`|Path to a folder container GeoJSON files with polygons that demarcate areas you care about|Working directory|
|`--cache-db PATH`|Path to the cache database (used to avoid double-posting)|`_cache` folder in workdir|
|`--workplace-token TOKEN`|Workplace token for the bot (must have _Message Any Member_ and _Group Chat Bot_ permissions)|If not given, will only print out|
|`--workplace-group ID`|Workplace Group ID to post in|Idem|
