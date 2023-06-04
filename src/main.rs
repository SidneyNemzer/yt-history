mod model;
mod parser;
mod pipe;
mod utf8_reader;

use colored::Colorize;
use std::fs::File;
use std::io::prelude::*;
use std::time::Instant;

use crate::model::{Models, WhereVideo, WhereWatched};
use crate::parser::ParseError;

const USE_CACHE: bool = false;
const DATA_PATH: &str = "data/watch-history.html";
const CACHE_PATH: &str = "data/cache.json";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let models = match load_models() {
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
    let mut counts = video_watches.iter().collect::<Vec<_>>();
    counts.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    println!();
    println!("{}", "Top 10 most watched videos".bold());
    for (i, (_, (count, video))) in counts.iter().enumerate().take(10) {
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

    Ok(())
}

fn load_models() -> Result<Models> {
    if !USE_CACHE {
        println!(
            "{}",
            "Not using cache because constant USE_CACHE is false".yellow()
        );
        return parse(DATA_PATH);
    }

    // Try loading cache
    return load_cache().or_else(|e| {
        // Fallback to parsing data from source file
        println!(
            "{} {}",
            "Couldn't use cache data:".dimmed(),
            e.to_string().dimmed()
        );

        let models = parse(DATA_PATH)?;

        let mut file = File::create(CACHE_PATH)?;
        write!(file, "{}", models.to_string())?;
        println!("{} {}", "Wrote cache to".dimmed(), CACHE_PATH.white());

        Ok(models)
    });
}

fn load_cache() -> Result<Models> {
    let start = Instant::now();
    let mut file = File::open(CACHE_PATH)?;
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
    let start = Instant::now();

    let result = parser::parse_file(file_path, parser::ParserType::Html);
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
        ParseError::IoError { location: _, error } => {
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
