mod model;

use chrono::TimeZone;
use model::{Models, WhereVideo};
use std::error;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Bytes;
use std::iter::Enumerate;
use std::iter::Peekable;
use std::time::Instant;

const USE_CACHE: bool = false;
const DATA_PATH: &str = "data/watch-history.html";
const CACHE_PATH: &str = "data/cache.json";

const ANCHOR_OPENING_TO_HREF: &str = "Watched\u{a0}<a href=\"";

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

type Iter = Peekable<Enumerate<Bytes<BufReader<File>>>>;

fn main() -> Result<()> {
    let models = load_models()?;

    println!("Found {} videos", models.count_videos(WhereVideo::Any));

    Ok(())
}

fn load_models() -> Result<Models> {
    if !USE_CACHE {
        println!("Not using cache because constant USE_CACHE is false");
        return parse(DATA_PATH);
    }

    // Try loading cache
    return match File::open(CACHE_PATH) {
        Ok(mut file) => {
            let start = Instant::now();

            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            let models = Models::from_str(contents);

            println!("Loaded cache data in {:?}", start.elapsed());

            Ok(models)
        }
        Err(error) => {
            // Fallback to parsing data from source file
            println!("Couldn't use cache data: {}", error);

            let models = parse(DATA_PATH)?;

            let mut file = File::create(CACHE_PATH)?;
            write!(file, "{}", models.to_string())?;
            println!("Wrote cache to {}", CACHE_PATH);

            Ok(models)
        }
    };
}

fn parse(file_path: &str) -> Result<Models> {
    let start = Instant::now();

    let f = File::open(file_path)?;
    let reader = BufReader::new(f);
    let mut bytes = reader.bytes().enumerate().peekable();

    let mut models = Models::new();

    loop {
        match read_data_row(&mut bytes) {
            Ok(data_row) => {
                let data_row_copy = data_row.clone();
                let channel =
                    models.find_or_create_channel(&data_row.channel_url, &data_row.channel_name);
                let video = models.find_or_create_video(data_row.url, data_row.title, channel);

                // Jun 29, 2021, 4:49:36 PM EDT
                // Aug 9, 2019, 4:26:40 PM EDT
                let date = chrono::Utc
                    .datetime_from_str(
                        filter_ascii(&data_row.date).as_str(),
                        "%h %e, %Y, %I:%M:%S%p %Z",
                    )
                    .expect(format!("Couldn't parse date from {:#?}", data_row_copy).as_str());

                models.insert_watched(date, WhereVideo::Reference(video));
            }
            Err(e) => {
                if let Some(ParseError::UnterminatedInput {
                    expected: _,
                    position: _,
                }) = e.downcast_ref::<ParseError>()
                {
                    if models.count_videos(WhereVideo::Any) == 0 {
                        // If no videos were found, return an error
                        return Err(e);
                    }

                    // Otherwise, consider parsing complete
                    break;
                } else {
                    return Err(e);
                }
            }
        }
    }

    println!("Parsed data in {:?}", start.elapsed());
    Ok(models)
}

fn filter_ascii(string: &String) -> String {
    string.chars().filter(|c| c.is_ascii()).collect()
}

#[derive(Debug, Clone, Default)]
struct DataRow {
    url: String,
    title: String,
    channel_name: String,
    channel_url: String,
    date: String,
}

fn read_data_row(bytes: &mut Iter) -> Result<DataRow> {
    let mut result = DataRow::default();

    skip_to(bytes, ANCHOR_OPENING_TO_HREF.into())?;

    result.url = read_until(bytes, '"'.into())?;

    skip_to(bytes, ">".into())?;

    result.title = read_until(bytes, "<".into())?.replace("\n", " ");

    // Skip to just before the channel link; it may be missing if the video is
    // no longer available.
    skip_to(bytes, "<br />".into())?;

    match peek(bytes)? {
        '<' => {
            // Parse channel

            skip_to(bytes, '"'.into())?;

            result.channel_url = read_until(bytes, '"'.into())?;

            skip_to(bytes, ">".into())?;

            result.channel_name = read_until(bytes, "<".into())?.replace("\n", " ");

            skip_to(bytes, "<br />".into())?;
        }
        'W' => {
            // Sometimes, the channel is missing and instead it has the text
            // "Watched at <time>". We skip this to the timestamp.
            skip_to(bytes, "<br />".into())?;
        }
        _ => {
            // No channel, just parse the date next.
        }
    }

    result.date = read_until(bytes, "\n".into())?
        .replace("\u{a0}", " ")
        .replace("\n", " ");

    Ok(result)
}

// Returns the next char without advancing the iterator.
fn peek(bytes: &mut Iter) -> Result<char> {
    let starting_byte_index = iter_index(bytes);

    match bytes.peek() {
        Some((_, maybe_byte)) => {
            let result = maybe_byte;
            let byte = match result {
                Ok(byte) => byte,
                Err(e) => {
                    // Copy io::Error. No idea why e.clone() doesn't work, but
                    // that just creates another &Error. *e doesn't work because
                    // io::Error doesn't implement Copy.
                    let e = std::io::Error::new(e.kind(), e.to_string());
                    return Err(e.into());
                }
            };
            Ok(byte.clone() as char)
        }
        None => {
            return Err(ParseError::UnterminatedInput {
                expected: "1 more byte".into(),
                position: starting_byte_index,
            }
            .into())
        }
    }
}

// Builds a string from bytes until the given string is found. If the string
// isn't found, the returned string will be the contents of bytes.
fn read_until(bytes: &mut Iter, string: String) -> Result<String> {
    let starting_byte_index = iter_index(bytes);

    // index into the string
    let mut i = 0;

    let mut result = String::new();

    for (_, maybe_byte) in bytes {
        let byte = maybe_byte?;

        if string.bytes().nth(i).unwrap() == byte {
            i += 1;
            if i == string.len() {
                return Ok(result);
            }
        } else {
            i = 0;
        }

        result.push(byte as char);
    }

    return Err(ParseError::UnterminatedInput {
        expected: string,
        position: starting_byte_index,
    }
    .into());
}

// Consumes bytes from the file until the given string is found. Returns
// ParseError if the string isn't found.
fn skip_to(bytes: &mut Iter, string: String) -> Result<()> {
    let starting_byte_index = iter_index(bytes);

    // index into the string; incremented as we see the correct bytes
    let mut i = 0;

    for (_, maybe_byte) in bytes {
        let byte = maybe_byte?;

        if string.bytes().nth(i).unwrap() == byte {
            i += 1;
            if i == string.len() {
                return Ok(());
            }
        } else {
            i = 0;
        }
    }

    return Err(ParseError::UnterminatedInput {
        expected: string,
        position: starting_byte_index,
    }
    .into());
}

// Returns the index of the next byte in bytes (without advancing the iterator).
fn iter_index(bytes: &mut Iter) -> usize {
    match bytes.peek() {
        Some((index, _)) => *index,
        None => panic!("bytes is not peekable"),
    }
}

//
// Errors
//

#[derive(Debug, Clone)]
enum ParseError {
    UnterminatedInput { expected: String, position: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid input: ")?;
        match self {
            ParseError::UnterminatedInput { expected, position } => write!(
                f,
                "unterminated input; expected {} after position {}",
                expected, position
            ),
        }
    }
}

impl error::Error for ParseError {}
