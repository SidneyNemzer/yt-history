mod model;
mod parser;
mod utf8_reader;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Instant;

use colored::Colorize;

use crate::model::{Models, WhereVideo, WhereWatched};
use crate::parser::ParseError;

const COMMAND_NAME: &str = "yt-history";
const USE_CACHE: bool = true;
const DEFAULT_DATA_PATH: &str = "data/watch-history.html";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.len() > 1 {
        println!("{} {}", "Error:".red(), "Too many arguments".bold());
        print_usage();
        std::process::exit(1);
    }

    let data_path: &str = args.get(0).map(|s| s.as_str()).unwrap_or(DEFAULT_DATA_PATH);

    let models = match load_models(data_path) {
        Ok(models) => models,
        Err(error) => {
            // ParseError is logged in parse(), only log other errors
            if error.downcast_ref::<ParseError>().is_none() {
                println!("{} {}", "Error:".red(), error);
            }

            std::process::exit(1);
        }
    };

    println!(
        "{} {} {} {} {}",
        "History contains".dimmed(),
        models.count_videos(WhereVideo::Any),
        "unique videos and".dimmed(),
        models.count_watches(WhereWatched::Any),
        "watches".dimmed(),
    );

    let video_watches = models.count_watched_by_video();
    let mut video_watch_counts = video_watches.iter().collect::<Vec<_>>();
    video_watch_counts.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    const COUNT: usize = 50;

    println!();
    println!(
        "{} {} {}",
        "Top".bold(),
        format!("{}", COUNT).bold(),
        "most watched videos".bold()
    );
    for (i, (_, (count, video))) in video_watch_counts.iter().enumerate().take(COUNT) {
        let s = if *count != 1 { "s" } else { "" };

        println!(
            "  {index}. {title} {viewed} {count} {time}{s}",
            index = i + 1,
            title = video.title,
            viewed = "viewed".dimmed(),
            count = count,
            time = "time".dimmed(),
            s = s.dimmed(),
        );
    }

    let channel_watches = models.count_watched_by_channel();
    let mut channel_watch_counts = channel_watches.iter().collect::<Vec<_>>();
    channel_watch_counts.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    println!();
    println!(
        "{} {} {}",
        "Top".bold(),
        format!("{}", COUNT).bold(),
        "most watched channels".bold()
    );
    for (i, (_, (count, channel))) in channel_watch_counts.iter().enumerate().take(COUNT) {
        let s = if *count != 1 { "s" } else { "" };

        println!(
            "  {index}. {title} {viewed} {count} {time}{s}",
            index = i + 1,
            title = channel.name,
            viewed = "viewed".dimmed(),
            count = count,
            time = "time".dimmed(),
            s = s.dimmed(),
        );
    }

    println!();
    println!("{}", "Top channel views by year".bold());

    let channel_watches_by_year = models.count_watched_by_channel_by_year();
    let mut channel_watches_by_year = channel_watches_by_year.iter().collect::<Vec<_>>();
    channel_watches_by_year.sort_by(|a, b| a.0.cmp(&b.0));

    for (year, channel_watches) in channel_watches_by_year.iter_mut() {
        let mut channel_watches = channel_watches.iter().collect::<Vec<_>>();
        channel_watches.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

        print!("{}: ", year);

        for (_, (count, channel)) in channel_watches.iter().take(10) {
            print!("{} {} ", channel.name, format!("({})", count).dimmed());
        }

        println!();
    }

    Ok(())
}

fn load_models(data_path_str: &str) -> Result<Models> {
    if !USE_CACHE {
        println!(
            "{}",
            "Not using cache because constant USE_CACHE is false".yellow()
        );
        return parse(data_path_str);
    }

    let data_path = Path::new(data_path_str);
    let data_filename = data_path.file_name().unwrap().to_str().unwrap();
    let cache_path = Path::new(data_path_str)
        .parent()
        .unwrap()
        .join(data_filename.to_owned() + ".cache.json");

    // Try loading cache
    return load_cache(&cache_path).or_else(|e| {
        // Fallback to parsing data from source file
        println!(
            "{} {}",
            "Couldn't use cache data:".dimmed(),
            e.to_string().dimmed()
        );

        let models = parse(data_path_str)?;

        let mut file = File::create(&cache_path)?;
        write!(file, "{}", models.to_string())?;
        println!(
            "{} {}",
            "Wrote cache to".dimmed(),
            cache_path.to_str().unwrap().white()
        );

        Ok(models)
    });
}

fn print_usage() {
    println!("Usage: {} [file]", COMMAND_NAME);
}

fn load_cache(cache_path: &PathBuf) -> Result<Models> {
    let start = Instant::now();
    let mut file = File::open(cache_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let models = Models::from_str(contents)?;
    println!(
        "{} {:.2?}",
        "Loaded cache data in".dimmed(),
        start.elapsed()
    );

    Ok(models)
}

fn parse(file_path: &str) -> Result<Models> {
    println!("{} {}", "Reading file".dimmed(), file_path.bold());

    let file_type = if file_path.ends_with(".json") {
        parser::ParserType::Json
    } else {
        parser::ParserType::Html
    };

    let start = Instant::now();

    let result = parser::parse_file(file_path, file_type);
    match result {
        Ok(models) => {
            println!("{} {:.2?}", "Parsed data in".dimmed(), start.elapsed());

            Ok(models)
        }
        Err(e) => {
            println!("{} {:.2?}", "Errored in".dimmed(), start.elapsed());

            if let Some(e) = e.downcast_ref::<ParseError>() {
                println!("{} {}", "Error parsing file".red(), file_path.bold());

                print_parse_error(e);
            }

            Err(e)
        }
    }
}

fn print_parse_error(error: &ParseError) {
    match error {
        ParseError::UnterminatedInput { expected, closest } => {
            println!("Unterminated input (file ends too soon)");
            println!("Expected: {}", expected);
            println!(
                "Non-ASCII bytes: {:?}",
                expected
                    .chars()
                    .filter(|c| !c.is_ascii())
                    .collect::<String>()
            );
            if let Some((closest, location)) = closest {
                println!(
                    "Closest: {} at line {} column {}",
                    closest,
                    location.lines + 1,
                    location.columns + 1
                );
            }
        }
        ParseError::InvalidUtf8 { location, bytes } => {
            println!(
                "Invalid UTF8 at line {} column {}",
                location.lines, location.columns
            );
            println!("Bytes: {:?}", bytes);
        }
        ParseError::IoError { error, .. } => {
            println!("IO error: {}", error);
        }
        ParseError::DateParseError {
            location,
            invalid_date,
            error,
        } => {
            println!(
                "Error parsing date {} at line {} column {}",
                invalid_date.bold(),
                location.lines,
                location.columns
            );

            let non_ascii = invalid_date.chars().filter(|c| !c.is_ascii());

            if non_ascii.clone().count() > 0 {
                const LEFT_PADDING_LEN: usize = 19;
                print!("{}", " ".repeat(LEFT_PADDING_LEN));

                for char in invalid_date.chars() {
                    if !char.is_ascii() {
                        print!("{}", "↑".yellow());
                    } else {
                        print!(" ");
                    }
                }
                println!();
                println!(
                    "{} {:x?}",
                    "hint: non-ASCII characters:".yellow(),
                    non_ascii.collect::<Vec<_>>()
                );
            }

            println!("{}", error);
        }
        ParseError::NoRows => {
            println!("No rows found");
        }
    }
}
