# RSS feed builder from git-log

This tool reads the history from one or multiple files from git and builds an
RSS feed. Commits they contain `no-rss` in a whole line in their description are
excluded.

# Usage

``` shellsession
% cargo run -q -- --help
gitlog2rss 0.1.0
Jörg Sommer <joerg@jo-so.de>
Create an RSS feed from git log

USAGE:
    gitlog2rss [FLAGS] [OPTIONS] <PATH>... --conf <FILE>

ARGS:
    <PATH>...    Path of the source file

FLAGS:
    -d, --debug      Print debug messages
    -h, --help       Prints help information
    -y, --pretty     Pretty print output
    -V, --version    Prints version information

OPTIONS:
    -c, --conf <FILE>        config file
    -p, --prefix <PREFIX>    PREFIX gets removed from the beginning of file names
```

## Config file

``` yaml
#
# See https://validator.w3.org/feed/docs/rss2.html for a description of
# these fields

repo: /home/joerg/website/.git
base-url: https://jo-so.de/
# beginning of paths that should be removed before using in URLs
strip-prefix: src/
ignore-files:
  - '**/index.md'
  - 'src/lib'
  - '**/_*'

item-title-page-new: Seite /%p erstellt
item-title-page-removed: Seite /%p gelöscht
item-title-page-modified: Seite /%p bearbeitet

channel-title: Am Interneteingang 8
channel-description: Webseite von Jörg Sommer
channel-link: https://jo-so.de/

language: de-de
# TTL in minutes, units like d/days, w/weeks, M/months are possible
# see https://docs.rs/humantime/latest/humantime/fn.parse_duration.html
ttl: 2d
# NOT IMPLEMENTED categories:
# NOT IMPLEMENTED   -

copyright: © 2017–2020 Jörg Sommer <joerg@jo-so.de>
managing-editor: joerg@jo-so.de (Jörg Sommer)
webmaster: technik@jo-so.de (Jörg Sommer)

generator: gitlog2rss

# when you might never publish new items
# http://backend.userland.com/skipHoursDays
# skip-hours: [0, 1, 2, 3, 4, 5, 6, 7, 8]
# skip-days: []

# for podcasts:
# NOT IMPLEMENTED rating: The PICS rating for the channel.
# NOT IMPLEMENTED image:
# NOT IMPLEMENTED text-input:
```

## Example for a single page

This examples uses the shell operator `<<<` to append the string to the config
file and feed all to *gitlog2rss* as config.

``` shellsession
% RUST_LOG_TIMESTAMP= cargo run -q -- -c - src/2020-02/Maxima.md -dy <../website/gitlog2rss.yaml <<<'channel-title: Am Interneteingang 8 -- Mathematik mit Maxima
channel-link: https://jo-so.de/2020-02/Maxima.html'
[INFO  gitlog2rss] Going to read config from stdin
[INFO  gitlog2rss] using path filter src/2020-02/Maxima.md
[INFO  gitlog2rss] Opening git repository /home/joerg/website/.git
[INFO  gitlog2rss] Skipping commit 395d434ffe13dad77d3058ade3be3a059bbd3a58, because of "no-rss"
[TRACE gitlog2rss] 18aea4feebd981347236d49fa5e6c84ad634b433 Added Some("src/2020-02/Maxima.md"), Some("src/2020-02/Maxima.md")
[DEBUG gitlog2rss] New rss item for 18aea4feebd981347236d49fa5e6c84ad634b433:src/2020-02/Maxima.md
[TRACE gitlog2rss] a3cbc8b0b0cb23407131b0125803a242c6c50ecd Modified Some("src/2020-02/Maxima.md"), Some("src/2020-02/Maxima.md")
[DEBUG gitlog2rss] New rss item for a3cbc8b0b0cb23407131b0125803a242c6c50ecd:src/2020-02/Maxima.md
[INFO  gitlog2rss] Skipping commit e891e94849de9a1cadac38fbf0f88df437e0d8cd, because of "no-rss"
[INFO  gitlog2rss] Skipping commit 9c311fd04617ce8d36824f65788b3a5c40e0eb35, because of "no-rss"
<rss version="2.0">
  <channel>
    <title>Am Interneteingang 8 -- Mathematik mit Maxima</title>
    <link>https://jo-so.de/2020-02/Maxima.html</link>
    <description>Webseite von Jörg Sommer</description>
    <language>de-de</language>
    <copyright>© 2017–2020 Jörg Sommer &lt;joerg@jo-so.de&gt;</copyright>
    <managingEditor>joerg@jo-so.de (Jörg Sommer)</managingEditor>
    <webMaster>technik@jo-so.de (Jörg Sommer)</webMaster>
    <pubDate>Fri, 03 Apr 2020 09:57:30 +0200</pubDate>
    <lastBuildDate>Mon, 20 Apr 2020 12:36:16 +0200</lastBuildDate>
    <generator>gitlog2rss</generator>
    <ttl>2880</ttl>
    <item>
      <title>Seite /2020-02/Maxima.html erstellt</title>
      <link>https://jo-so.de/2020-02/Maxima.html</link>
      <author>joerg@jo-so.de (Jörg Sommer)</author>
      <pubDate>Fri, 03 Apr 2020 09:57:30 +0200</pubDate>
    </item>
    <item>
      <title>Seite /2020-02/Maxima.html bearbeitet</title>
      <link>https://jo-so.de/2020-02/Maxima.html</link>
      <author>joerg@jo-so.de (Jörg Sommer)</author>
      <pubDate>Mon, 20 Apr 2020 12:36:16 +0200</pubDate>
    </item>
  </channel>
</rss>```

## Example with the whole website

``` shellsession
% /path/to/gitlog2rss/target/release/gitlog2rss \
  -c gitlog2rss.yaml 'src/**/*.md' > www/rss
```

# Further reading

* <https://validator.w3.org/feed/docs/rss2.html>
* <https://validator.w3.org/feed/check.cgi>

# TODO

* add [anyhow](https://crates.io/crates/anyhow) for better error messages;
  [Getting line numbers with `?` as I would with
  `unwrap()`](https://users.rust-lang.org/t/getting-line-numbers-with-as-i-would-with-unwrap/47002)
