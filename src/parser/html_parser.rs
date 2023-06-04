use std::io::Read;
use std::iter::Enumerate;
use std::iter::Peekable;

use chrono::TimeZone;

use crate::model::{Models, WhereVideo};
use crate::utf8_reader;
use crate::utf8_reader::Utf8Iter;

type Iter<R> = Peekable<Enumerate<Utf8Iter<R>>>;

// U+00A0 is a non-breaking space
const ANCHOR_OPENING_TO_HREF: &str = "Watched\u{00A0}<a href=\"";

// Examples:
// Jun 29, 2021, 4:49:36 PM EDT
// Aug 9, 2019, 4:26:40 PM EDT
//
// U+202F is a narrow non-breaking space
const DATE_FORMAT: &str = "%h %e, %Y, %I:%M:%S\u{202F}%p %Z";

pub struct ModelsParser {
    models: Models,
    line: usize,
    column: usize,
    chars_read: usize,
}

#[derive(Debug, Clone)]
struct DataRow {
    url: String,
    title: String,
    channel_name: String,
    channel_url: String,
    date: chrono::DateTime<chrono::Utc>,
}

impl Default for DataRow {
    fn default() -> Self {
        Self {
            url: String::new(),
            title: String::new(),
            channel_name: String::new(),
            channel_url: String::new(),
            date: chrono::DateTime::<chrono::Utc>::MIN_UTC,
        }
    }
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

    pub fn parse<R: Read>(&mut self, raw_chars: Utf8Iter<R>) -> Result<(), ParseError> {
        let mut chars = raw_chars.enumerate().peekable();

        // Ensure at least one row can be read
        match self.next_data_row(&mut chars)? {
            Some(row) => {
                self.insert_row(row)?;
            }
            None => {
                return Err(ParseError::NoRows);
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

    pub fn location(&self) -> Location {
        Location {
            chars: self.chars_read,
            columns: self.column(),
            lines: self.line(),
        }
    }

    fn insert_row(&mut self, row: DataRow) -> Result<(), ParseError> {
        let channel = self
            .models
            .find_or_create_channel(&row.channel_url, &row.channel_name);
        let video = self
            .models
            .find_or_create_video(row.url, row.title, channel);

        self.models
            .insert_watched(row.date, WhereVideo::Reference(video));

        Ok(())
    }

    fn next_data_row<R: Read>(
        &mut self,
        chars: &mut Iter<R>,
    ) -> Result<Option<DataRow>, ParseError> {
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

        let date_string = self.read_until(chars, "\n")?;
        row.date = chrono::Utc
            .datetime_from_str(date_string.as_str(), DATE_FORMAT)
            .map_err(|error| ParseError::DateParseError {
                location: self.location(),
                invalid_date: date_string,
                error,
            })?;

        Ok(Some(row))
    }

    fn skip_to<R: Read>(&mut self, chars: &mut Iter<R>, s: &str) -> Result<(), ParseError> {
        let mut closest = String::with_capacity(s.len());
        let mut closest_location = Location::default();

        let mut found = String::with_capacity(s.len());
        let mut found_location = Location::default();

        for (_, maybe_char) in chars {
            let char = maybe_char.map_err(|e| ParseError::from_utf8_error(&e, self.location()))?;
            self.chars_read += 1;

            if char == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }

            let want = match s.chars().nth(found.chars().count()) {
                Some(c) => c,
                None => {
                    panic!(
                        "skip_to internal desync: `found` is longer than s (s: {}, found: {})",
                        s, found
                    )
                }
            };

            if want == char {
                if found.len() == 0 {
                    found_location = Location {
                        lines: self.line,
                        columns: self.column,
                        chars: self.chars_read,
                    };
                }

                found.push(char);

                if &found == s {
                    return Ok(());
                }
            } else {
                if found.len() > 0 {
                    found.push(char);
                    if found.chars().count() > closest.chars().count() {
                        closest = found.clone();
                        closest_location = found_location.clone();
                    }
                }

                found.clear();
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

    fn read_until<R: Read>(&mut self, chars: &mut Iter<R>, s: &str) -> Result<String, ParseError> {
        let mut closest = String::with_capacity(s.len());
        let mut closest_location = Location::default();

        let mut found = String::with_capacity(s.len());
        let mut found_location = Location::default();

        let mut read = String::new();

        for (_, maybe_char) in chars {
            let char = maybe_char.map_err(|e| ParseError::from_utf8_error(&e, self.location()))?;
            self.chars_read += 1;

            if char == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }

            let want = match s.chars().nth(found.chars().count()) {
                Some(c) => c,
                None => {
                    panic!(
                        "read_until internal desync: `found` is longer than s (s: {}, found: {})",
                        s, found
                    )
                }
            };

            if want == char {
                // Found part of string, don't add this to read

                if found.len() == 0 {
                    found_location = Location {
                        lines: self.line,
                        columns: self.column,
                        chars: self.chars_read,
                    };
                }

                found.push(char);

                if &found == s {
                    return Ok(read);
                }
            } else {
                if found.len() > 0 {
                    found.push(char);

                    if found.chars().count() > closest.chars().count() {
                        closest = found.clone();
                        closest_location = found_location.clone();
                    }

                    // We found part of the string, but then it didn't match.
                    // Append what we saw to read.
                    push_collapse_whitespace(&mut read, &found);

                    found.clear();
                } else {
                    push_collapse_whitespace(&mut read, &String::from(char));
                }
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

    fn peek<R: Read>(&mut self, chars: &mut Iter<R>) -> Result<char, ParseError> {
        match chars.peek() {
            Some((_, Ok(char))) => Ok(*char),
            Some((_, Err(e))) => Err(ParseError::from_utf8_error(e, self.location())),
            None => Err(ParseError::UnterminatedInput {
                expected: "any character".into(),
                closest: None,
            }),
        }
    }
}

/// Appends s to target, converting whitespace characters to U+0020 SPACE.
/// Consecutive whitespace is collapsed.
fn push_collapse_whitespace(target: &mut String, s: &str) {
    let mut last_whitespace = target
        .chars()
        .last()
        .map(|c| c.is_whitespace())
        .unwrap_or(false);

    for c in s.chars() {
        if c.is_whitespace() {
            if !last_whitespace {
                target.push(' ');
            }
            last_whitespace = true;
        } else {
            target.push(c);
            last_whitespace = false;
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
        location: Location,
        bytes: Vec<u8>,
    },
    IoError {
        location: Location,
        error: String,
    },
    DateParseError {
        location: Location,
        invalid_date: String,
        error: chrono::ParseError,
    },
    NoRows,
}

impl ParseError {
    fn from_utf8_error(error: &utf8_reader::Error, location: Location) -> ParseError {
        match error {
            utf8_reader::Error::InvalidBytes(bytes) => ParseError::InvalidUtf8 {
                location,
                bytes: bytes.to_vec(),
            },
            utf8_reader::Error::IoError(error) => ParseError::IoError {
                location,
                error: error.to_string(),
            },
            utf8_reader::Error::End => ParseError::UnterminatedInput {
                expected: "1 more character".into(),
                closest: None,
            },
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}

impl std::error::Error for ParseError {}
