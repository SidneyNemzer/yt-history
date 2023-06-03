use std::fs::File;
use std::io::BufReader;
use std::iter::Enumerate;
use std::iter::Peekable;

use chrono::TimeZone;

use crate::model::{Models, WhereVideo};
use crate::utf8_reader::{NextUtf8, Utf8Iter};

type Iter = Peekable<Enumerate<Utf8Iter<BufReader<File>>>>;

// U+00A0 is a non-breaking space
const ANCHOR_OPENING_TO_HREF: &str = "Watched\u{00A0}<a href=\"";

pub struct ModelsParser {
    models: Models,
    line: usize,
    column: usize,
    chars_read: usize,
}

#[derive(Debug, Clone, Default)]
struct DataRow {
    url: String,
    title: String,
    channel_name: String,
    channel_url: String,
    date: String,
}

#[derive(Debug, Default, Clone)]
pub struct Location {
    pub chars: usize,
    pub columns: usize,
    pub lines: usize,
}

impl ModelsParser {
    pub fn new() -> Self {
        Self {
            models: Models::new(),
            line: 0,
            column: 0,
            chars_read: 0,
        }
    }

    pub fn parse(&mut self, raw_chars: Utf8Iter<BufReader<File>>) -> Result<(), ParseError> {
        let mut chars = raw_chars.enumerate().peekable();

        // Ensure at least one row can be read
        match self.next_data_row(&mut chars)? {
            Some(row) => {
                self.insert_row(row)?;
            }
            None => {
                return Err(ParseError::UnterminatedInput {
                    expected: "data row".to_string(),
                    closest: None,
                });
            }
        };

        loop {
            match self.next_data_row(&mut chars)? {
                Some(row) => {
                    self.insert_row(row)?;
                }
                None => {
                    // No more rows
                    return Ok(());
                }
            }
        }
    }

    pub fn to_models(self) -> Models {
        self.models
    }

    /// The current line of the parser, starting at 1.
    pub fn line(&self) -> usize {
        self.line + 1
    }

    /// The current column of the parser, starting at 1.
    pub fn column(&self) -> usize {
        self.column + 1
    }

    fn insert_row(&mut self, row: DataRow) -> Result<(), ParseError> {
        let channel = self
            .models
            .find_or_create_channel(&row.channel_url, &row.channel_name);
        let video = self
            .models
            .find_or_create_video(row.url, row.title, channel);

        // Examples:
        // Jun 29, 2021, 4:49:36 PM EDT
        // Aug 9, 2019, 4:26:40 PM EDT
        //
        // U+202F is a narrow non-breaking space
        const DATE_FORMAT: &str = "%h %e, %Y, %I:%M:%S\u{202F}%p %Z";

        let date = chrono::Utc
            .datetime_from_str(row.date.as_str(), DATE_FORMAT)
            .map_err(|error| ParseError::DateParseError {
                invalid_date: row.date,
                error,
            })?;

        self.models
            .insert_watched(date, WhereVideo::Reference(video));

        Ok(())
    }

    fn next_data_row(&mut self, chars: &mut Iter) -> Result<Option<DataRow>, ParseError> {
        let mut row = DataRow::default();

        let skip_result = self.skip_to(chars, ANCHOR_OPENING_TO_HREF.into());
        match skip_result {
            Ok(()) => {}
            Err(ParseError::UnterminatedInput {
                expected: _,
                closest: _,
            }) => {
                // Opening wasn't found before EOF, treat this as containing no
                // more rows.
                return Ok(None);
            }
            Err(e) => return Err(e),
        }

        row.url = self.read_until(chars, "\"")?;
        self.skip_to(chars, ">")?;
        row.title = self.read_until(chars, "<")?;
        // Skip just before the channel link
        self.skip_to(chars, "<br />")?;

        match self.peek(chars)? {
            '<' => {
                // Parse channel
                self.skip_to(chars, "\"")?;
                row.channel_url = self.read_until(chars, "\"")?;
                self.skip_to(chars, ">")?;
                row.channel_name = self.read_until(chars, "<")?;
                self.skip_to(chars, "<br />")?;
            }
            'W' => {
                // Sometimes, the channel is missing and instead it has the text
                // "Watched at <time>". We skip this text to the start of the
                // timestamp.
                self.skip_to(chars, "<br />")?;
            }
            _ => (),
        }

        row.date = self.read_until(chars, "\n")?;

        Ok(Some(row))
    }

    fn skip_to(&mut self, chars: &mut Iter, s: &str) -> Result<(), ParseError> {
        let mut closest = String::new();
        let mut closest_location = Location::default();
        let mut s_index = 0;

        let mut longest_closest = String::new();
        let mut longest_closest_location = Location::default();

        for (_, maybe_char) in chars {
            let char = std::result::Result::<char, ParseError>::from(maybe_char)?;
            self.chars_read += 1;

            if char == 'W' {
                // println!("{}:{} {}", self.line + 1, self.column + 1, char);
            }

            if char == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }

            let want = s.chars().nth(s_index).expect(&format!(
                "skip_to: s_index out of sync with s (s: {:?}, s_index: {})",
                s, s_index
            ));

            if want == char {
                if s_index == 0 {
                    closest = String::new();
                    closest_location = Location {
                        chars: self.chars_read,
                        columns: self.column,
                        lines: self.line,
                    };
                }

                s_index += 1;
                closest.push(char);

                if s_index == s.chars().count() {
                    return Ok(());
                }
            } else {
                if s_index > 0 {
                    closest.push(char);
                    if closest.len() > longest_closest.len() {
                        longest_closest = closest.clone();
                        longest_closest_location = closest_location.clone();
                    }
                }

                s_index = 0;
            }
        }

        Err(ParseError::UnterminatedInput {
            expected: s.into(),
            closest: if closest.is_empty() {
                None
            } else {
                Some((closest, closest_location))
            },
        })
        .into()
    }

    fn read_until(&mut self, chars: &mut Iter, s: &str) -> Result<String, ParseError> {
        let mut read = String::new();
        let mut closest = String::new();
        let mut s_index = 0;

        for (_, maybe_char) in chars {
            let char = std::result::Result::<char, ParseError>::from(maybe_char)?;
            self.chars_read += 1;

            if char == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }

            let want = s
                .chars()
                .nth(s_index)
                .expect("read_until: s_index out of sync with s");

            if want == char {
                // Found part of string, don't add this to read

                if s_index == 0 {
                    closest = String::new();
                }

                s_index += 1;

                if s_index == s.chars().count() {
                    return Ok(read);
                }
            } else {
                if s_index > 0 {
                    closest.push(char);

                    // We found part of the string, but then it didn't match. Append what we saw to read.
                    read.push_str(&closest);
                }

                s_index = 0;
                read.push(char);
            }
        }

        Err(ParseError::UnterminatedInput {
            expected: s.into(),
            closest: if closest.is_empty() {
                None
            } else {
                Some((closest, todo!("track and return location of closest")))
            },
        })
        .into()
    }

    fn peek(&mut self, chars: &mut Iter) -> Result<char, ParseError> {
        match chars.peek() {
            Some((_, maybe_char)) => {
                let char = std::result::Result::<char, ParseError>::from(maybe_char.clone())?;
                Ok(char)
            }
            None => Err(ParseError::UnterminatedInput {
                expected: "any character".into(),
                closest: None,
            }),
        }
    }
}

//
// Errors
//

#[derive(Debug, Clone)]
pub enum ParseError {
    UnterminatedInput {
        expected: String,
        closest: Option<(String, Location)>,
    },
    InvalidUtf8 {
        bytes: Vec<u8>,
    },
    IoError {
        error: String,
    },
    DateParseError {
        invalid_date: String,
        error: chrono::ParseError,
    },
    NoRows,
}

impl From<NextUtf8> for std::result::Result<char, ParseError> {
    fn from(next: NextUtf8) -> Self {
        match next {
            NextUtf8::Valid(char) => Ok(char),
            NextUtf8::InvalidBytes(bytes) => Err(ParseError::InvalidUtf8 {
                bytes: bytes.to_vec(),
            }),
            NextUtf8::IoError(error) => Err(ParseError::IoError {
                error: error.to_string(),
            }),
            NextUtf8::End => Err(ParseError::UnterminatedInput {
                closest: None,
                expected: "1 more character".into(),
            }),
        }
    }
}
