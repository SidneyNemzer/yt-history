use std::error;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Bytes;

const FILE_PATH: &str = "data/watch-history.html";
const ANCHOR_OPENING_TO_HREF: &str = "Watched\u{a0}<a href=\"";

#[derive(Debug)]
struct Video {
    url: String,
    title: String,
    channel_url: String,
    channel_name: String,
    date: String,
}

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

type Iter = std::iter::Enumerate<Bytes<BufReader<File>>>;

fn main() -> Result<()> {
    let f = File::open(FILE_PATH)?;
    let reader = BufReader::new(f);
    let mut bytes = reader.bytes().enumerate();

    let mut videos = Vec::new();

    loop {
        let video_result = read_video_data(&mut bytes);

        match video_result {
            Ok(video) => videos.push(video),
            Err(e) => {
                if let Some(ParseError::UnterminatedInput {
                    expected: _,
                    position: _,
                }) = e.downcast_ref::<ParseError>()
                {
                    if videos.len() == 0 {
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

    println!("Found {} rows", videos.len());

    let mut watch_count_by_url = std::collections::HashMap::new();

    struct WatchCount {
        title: String,
        count: u32,
    }

    for video in videos {
        let watch_count = watch_count_by_url.entry(video.url).or_insert(WatchCount {
            title: video.title,
            count: 0,
        });
        watch_count.count += 1;
    }

    println!("Total videos: {}", watch_count_by_url.len());

    let mut watch_count: Vec<(&String, &WatchCount)> = watch_count_by_url.iter().collect();
    watch_count.sort_by(|a, b| b.1.count.cmp(&a.1.count));

    println!("Top 10 videos by watch count (n={})", watch_count.len());
    for (i, (url, watch_count)) in watch_count.iter().take(50).enumerate() {
        println!(
            "{}. {}: {} {}",
            i + 1,
            filter_ascii(&watch_count.title),
            watch_count.count,
            filter_ascii(*url),
        );
    }

    Ok(())
}

fn filter_ascii(string: &String) -> String {
    string.chars().filter(|c| c.is_ascii()).collect()
}

fn read_video_data(bytes: &mut Iter) -> Result<Video> {
    skip_to(bytes, ANCHOR_OPENING_TO_HREF.into())?;

    let url = read_until(bytes, '"'.into())?;

    skip_to(bytes, ">".into())?;

    let title = read_until(bytes, "<".into())?.replace("\n", " ");

    skip_to(bytes, '"'.into())?;

    let channel_url = read_until(bytes, '"'.into())?;

    skip_to(bytes, ">".into())?;

    let channel_name = read_until(bytes, "<".into())?.replace("\n", " ");

    skip_to(bytes, "<br />".into())?;

    let date = read_until(bytes, "\n".into())?
        .replace("\u{a0}", " ")
        .replace("\n", " ");

    Ok(Video {
        url,
        title,
        channel_name,
        channel_url,
        date,
    })
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
    // TODO this still consumes the byte. We need to change Iter to be Peekable
    // and call `peekable()` once.

    // match bytes.peekable().peek() {
    //     Some((index, _)) => *index,
    //     None => panic!("bytes is not peekable"),
    // }
    0
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
