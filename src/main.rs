use chrono::{
    FixedOffset,
    TimeZone,
};
use clap::{
    crate_authors,
    crate_description,
    crate_name,
    crate_version,
    App,
    Arg,
};
use git2::{
    Delta,
    DiffFindOptions,
    DiffOptions,
    Pathspec,
    PathspecFlags,
    Repository,
    Sort,
};
use log::{
    debug,
    info,
    trace,
    warn,
};
use rss::{
    ChannelBuilder,
    ItemBuilder,
};
use std::{
    env,
    error,
    fs,
    io::{self, Read},
};
use yaml_rust::{
    Yaml,
    YamlLoader,
};

fn rfc822_time(time: &git2::Time) -> String {
    FixedOffset::east(time.offset_minutes() * 60)
        .timestamp(time.seconds(), 0)
        .to_rfc2822()
}

fn main() -> Result<(), Box<dyn error::Error + 'static>> {
    let args = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!(", "))
        .about(crate_description!())
        .arg(
            Arg::new("conf")
                .short('c')
                .long("conf")
                .takes_value(true)
                .value_name("FILE")
                .required(true)
                .about("config file")
        ).arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .about("Print debug messages")
        ).arg(
            Arg::new("prefix")
                .short('p')
                .long("prefix")
                .takes_value(true)
                .value_name("PREFIX")
                .about("PREFIX gets removed from the beginning of file names")
        ).arg(
            Arg::new("pretty")
                .short('y')
                .long("pretty")
                .about("Pretty print output")
        ).arg(
            Arg::new("path")
                .value_name("PATH")
                .about("Path of the source file")
                .required(true)
                .multiple(true)
        ).get_matches();

    {
        let mut logger = env_logger::builder();
        match env::var("RUST_LOG_TIMESTAMP").as_deref() {
            Ok("sec") => { logger.format_timestamp_secs(); }
            Ok("micro") => { logger.format_timestamp_micros(); }
            Ok("milli") => { logger.format_timestamp_millis(); }
            Ok("nano") => { logger.format_timestamp_nanos(); }
            Ok(_) => { logger.format_timestamp(None); }
            _ => {},
        }

        if args.is_present("debug") {
            logger.filter_level(log::LevelFilter::Trace);
        }

        logger.init();
    }

    let conf = {
        let txt = match args.value_of("conf").unwrap() {
            "-" => {
                info!("Going to read config from stdin");
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                buf
            }
            path => {
                info!("Going to read config file {}", path);
                fs::read_to_string(path)?
            }
        };

        YamlLoader::load_from_str(&txt)?.pop().unwrap()
    };

    let mut diff_opts = DiffOptions::new();
    diff_opts.ignore_filemode(true)
        .ignore_submodules(true)
        .ignore_whitespace(true);

    for e in args.values_of("path").unwrap() {
        info!("using path filter {}", e);
        diff_opts.pathspec(e);
    }

    let mut diff_similar_opts = DiffFindOptions::default();
    diff_similar_opts.renames(true);

    let ignored_files = if let Some(list) = conf["ignore-files"].as_vec() {
        Some(Pathspec::new(list.iter().filter_map(|x| x.as_str()))?)
    } else {
        None
    };

    let repo = if let Some(path) = conf["repo"].as_str() {
        info!("Opening git repository {}", path);
        Repository::open(path)?
    } else {
        let repo = Repository::open_from_env()?;
        info!("Successfully opened git repository {}", repo.path().display());
        repo
    };

    let base_url = url::Url::parse(conf["base-url"].as_str().unwrap())?;
    let strip_prefix = args.value_of("prefix")
        .or_else(|| conf["strip-prefix"].as_str())
        .unwrap_or("");

    let mut items = Vec::new();

    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::REVERSE | Sort::TIME)?;
    revwalk.push_head()?;
    for id in revwalk {
        let commit = repo.find_commit(id?)?;
        if commit.parent_count() > 1 {
            debug!("Skipping merge commit {}", commit.id());
            continue;
        }
        if commit.message().map_or(false, |msg| msg.contains("\nno-rss\n")) {
            info!("Skipping commit {}, because of \"no-rss\"", commit.id());
            continue;
        }

        let author = commit.author();
        let author_date = rfc822_time(&author.when());
        let author = author.email().unwrap().to_string()
            + " (" + author.name().unwrap() + ")";

        let parent_tree = if commit.parent_count() == 1 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = repo.diff_tree_to_tree(
            parent_tree.as_ref(), Some(&commit.tree()?), Some(&mut diff_opts)
        )?;
        // to find renames or copies
        // diff.find_similar(Some(&mut diff_similar_opts))?;

        for delta in diff.deltas() {
            trace!("{} {:?} {:?}, {:?}",
                   commit.id(),
                   delta.status(),
                   delta.old_file().path(),
                   delta.new_file().path(),
            );

            let file;
            let text;
            match delta.status() {
                Delta::Added => {
                    file = delta.new_file();
                    text = "item-title-page-new";
                }

                Delta::Deleted => {
                    file = delta.old_file();
                    text = "item-title-page-removed";
                }

                Delta::Modified => {
                    file = delta.new_file();
                    text = "item-title-page-modified"
                }

                st => {
                    warn!(
                        "Unhandled diff state {:?} for commit {} between {:?} and {:?}",
                        st,
                        commit.id(),
                        delta.old_file().path(),
                        delta.new_file().path(),
                    );
                    continue;
                }
            }

            let path = file.path().unwrap();

            if let Some(ref ign) = ignored_files {
                if ign.matches_path(&path, PathspecFlags::default()) {
                    info!("Skipping delta of ignored file {} in commit {}",
                          path.display(), commit.id());
                    continue;
                }
            }

            let path = path.to_str().unwrap();
            let url_path = {
                let first = if path.starts_with(strip_prefix) { strip_prefix.len() } else { 0 };

                if path.ends_with(".md") {
                    path[first..path.len() - 2].to_string() + "html"
                } else {
                    path[first..].to_string()
                }
            };

            items.push(
                ItemBuilder::default()
                    .author(Some(author.clone()))
                // TODO .description(Some("Neue Seite erstellt".into()));
                // TODO .categories(vec![])
                // TODO .guid(Some(Guid))
                    .pub_date(Some(author_date.clone()))
                    .title(
                        conf[text].as_str().map(|title| title.replace("%p", &url_path))
                    )
                    .link(Some(base_url.join(&url_path)?.into_string()))
                    .build().unwrap()
            );
            debug!("New rss item for {}:{}", commit.id(), path)
        }
    }

    let chan = ChannelBuilder::default()
        .title(conf["channel-title"].as_str().unwrap())
        .link(conf["channel-link"].as_str().unwrap())
        .description(conf["channel-description"].as_str().unwrap())
        .pub_date(items.first().and_then(|x| x.pub_date()).map(|x| x.to_owned()))
        .last_build_date(items.last().and_then(|x| x.pub_date()).map(|x| x.to_owned()))
        .language(conf["language"].as_str().map(|x| x.to_owned()))
        .copyright(conf["copyright"].as_str().map(|x| x.to_owned()))
        .managing_editor(conf["managing-editor"].as_str().map(|x| x.to_owned()))
        .webmaster(conf["webmaster"].as_str().map(|x| x.to_owned()))
    // TODO .categories(vec![])
        .generator(conf["generator"].as_str().map(|x| x.to_owned()))
        .ttl(match &conf["ttl"] {
            Yaml::Integer(x) => Some(format!("{}", x)),
            Yaml::String(x) => Some(format!("{}", humantime::parse_duration(x)?.as_secs() / 60)),
            Yaml::BadValue => None,
            _ => return Err("Invalid value of config entry 'ttl'".into())
        })
        .skip_hours(
            conf["skip-hours"].as_vec()
                .map_or(
                    vec![],
                    |vec| vec.iter()
                        .filter_map(|x| x.as_i64())
                        .map(|x| format!("{}", x))
                        .collect()
                )
        )
        .skip_days(
            conf["skip-days"].as_vec()
                .map_or(
                    vec![],
                    |vec| vec.iter()
                        .filter_map(|x| x.as_i64())
                        .map(|x| format!("{}", x))
                        .collect()
                )
        )
        .items(items)
        .build()
        .unwrap();

    if args.is_present("pretty") {
        chan.pretty_write_to(&mut io::stdout(), b' ', 2)?;
        println!("");
    } else {
        chan.write_to(&mut io::stdout())?;
    }

    Ok(())
}
