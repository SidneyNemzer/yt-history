use std::io::Read;
use std::str;

// Ideas:
// - Buffer in a u32 or u64
// - Buffer in a Ring Buffer
// - Buffer in a Vec<u8>

/// The maximum number of bytes in a valid UTF-8 code point. Not all 32-bit
/// numbers are valid UTF-8 code points. Validation is done by the rust built-in
/// String type.
///
/// Source: https://www.unicode.org/versions/Unicode15.0.0/UnicodeStandard-15.0.pdf
/// Section 3.9, Definition D92
const MAX_BYTES_UTF8_CODE_POINT: usize = 4;

pub struct Utf8Iter<R>
where
    R: Read,
{
    bytes_read: usize,
    reader: R,
    buf: [u8; MAX_BYTES_UTF8_CODE_POINT],
    buf_len: usize,
}

#[derive(Debug)]
pub enum Error {
    /// An invalid UTF-8 character was read. Calling next() again skips the
    /// invalid bytes.
    InvalidBytes(Vec<u8>),
    /// The underlying reader returned an IO error.
    IoError(std::io::Error),
    /// The end of the stream was reached. next() may be called again because
    /// the underlying file descriptor may have data later. For example, a file
    /// could have data appended, or a socket could receive another packet.
    End,
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Self::InvalidBytes(b) => Self::InvalidBytes(b.clone()),
            Self::IoError(e) => Self::IoError(std::io::Error::new(e.kind(), e.to_string())),
            Self::End => Self::End,
        }
    }
}

impl PartialEq<Error> for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (Error::InvalidBytes(a), Error::InvalidBytes(b)) => a == b,
            (Error::IoError(a), Error::IoError(b)) => a.kind() == b.kind(),
            (Error::End, Error::End) => true,
            _ => false,
        }
    }
}

impl<R: Read> Utf8Iter<R> {
    pub fn new(reader: R) -> Utf8Iter<R> {
        Utf8Iter {
            bytes_read: 0,
            reader,
            buf: [0; MAX_BYTES_UTF8_CODE_POINT],
            buf_len: 0,
        }
    }

    /// Returns the next character in the stream, possibly made of several bytes
    /// from the underlying reader.
    ///
    /// NextUtf8::End indicates the underlying reader returned Ok(0), which
    /// means the end of the file descriptor was found -- for files, the end of
    /// the file. For sockets or pipes, the socket/pipe is empty. However next()
    /// could return data again in the future. Files can have data appended and
    /// sockets/pipes can receive data.
    pub fn next(&mut self) -> Result<char, Error> {
        loop {
            match self.reader.read(self.buf[self.buf_len..].as_mut()) {
                Ok(0) => {
                    if self.buf_len == 0 {
                        return Err(Error::End);
                    }
                }
                Ok(n) => {
                    self.buf_len += n;
                    self.bytes_read += n;
                }
                Err(e) => {
                    return Err(Error::IoError(e));
                }
            }

            match str::from_utf8(&self.buf[..self.buf_len]) {
                Ok(s) => {
                    let c = s.chars().next().unwrap();
                    self.buf.copy_within(c.len_utf8().., 0);
                    self.buf_len -= c.len_utf8();
                    return Ok(c);
                }
                Err(e) => {
                    if e.valid_up_to() > 0 {
                        // The beginning of the buffer is a valid code point, copy it out.
                        let code_points = &self.buf[..e.valid_up_to()];
                        let char = str::from_utf8(code_points).unwrap().chars().next().unwrap();

                        // Shift the buffer left by the number of bytes in the first char.
                        let removed_bytes = char.len_utf8();
                        let remaining_buffer_range = removed_bytes..MAX_BYTES_UTF8_CODE_POINT;
                        self.buf.copy_within(remaining_buffer_range, 0);
                        self.buf_len = self.buf_len - removed_bytes;

                        return Ok(char);
                    }

                    match e.error_len() {
                        Some(n) => {
                            // The beginning of the buffer is invalid, remove it.
                            let invalid_bytes = Vec::from(&self.buf[..n]);

                            self.buf.copy_within(n..MAX_BYTES_UTF8_CODE_POINT, 0);
                            self.buf_len = self.buf_len - n;

                            return Err(Error::InvalidBytes(invalid_bytes));
                        }
                        None => {
                            continue;
                        }
                    }
                }
            };
        }
    }
}

impl<R: Read> Iterator for Utf8Iter<R> {
    type Item = Result<char, Error>;

    fn next(&mut self) -> Option<Result<char, Error>> {
        match self.next() {
            Err(Error::End) => None,
            next => Some(next),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipe;
    use std::io::Write;

    #[test]
    fn test_empty_reader() {
        let (mut reader, mut writer) = pipe::async_pipe_buffered();

        let mut iter = Utf8Iter::new(&mut reader);
        assert_eq!(iter.next(), Err(Error::End));

        writer.write_all(vec![0x61].as_slice()).unwrap();
        assert_eq!(iter.next(), Ok('a'));
        assert_eq!(iter.next(), Err(Error::End));
    }

    #[test]
    fn test_len_1_reader() {
        let mut reader = std::io::Cursor::new(vec![0x61]);
        let mut iter = Utf8Iter::new(&mut reader);

        assert_eq!(iter.next(), Ok('a'));
        assert_eq!(iter.next(), Err(Error::End));
    }

    #[test]
    fn test_len_4_reader() {
        let mut reader = std::io::Cursor::new(vec![0x61, 0x61, 0x61, 0x61]);
        let mut iter = Utf8Iter::new(&mut reader);

        assert_eq!(iter.next(), Ok('a'));
        assert_eq!(iter.next(), Ok('a'));
        assert_eq!(iter.next(), Ok('a'));
        assert_eq!(iter.next(), Ok('a'));
        assert_eq!(iter.next(), Err(Error::End));
    }

    #[test]
    fn test_len_1_invalid_reader() {
        let mut reader = std::io::Cursor::new(vec![0xC0]);
        let mut iter = Utf8Iter::new(&mut reader);

        assert_eq!(iter.next(), Err(Error::InvalidBytes(vec![0xC0])));
        assert_eq!(iter.next(), Err(Error::End));
    }

    #[test]
    fn test_invalid_and_valid_reader() {
        let mut reader = std::io::Cursor::new(vec![0xE0, 0x61]);
        let mut iter = Utf8Iter::new(&mut reader);

        assert_eq!(iter.next(), Err(Error::InvalidBytes(vec![0xE0])));
        assert_eq!(iter.next(), Ok('a'));
        assert_eq!(iter.next(), Err(Error::End));
    }

    #[test]
    fn test_valid_and_invalid_reader() {
        let mut reader = std::io::Cursor::new(vec![0x61, 0x61, 0x61, 0xC0]);
        let mut iter = Utf8Iter::new(&mut reader);

        assert_eq!(iter.next(), Ok('a'));
        assert_eq!(iter.next(), Ok('a'));
        assert_eq!(iter.next(), Ok('a'));
        assert_eq!(iter.next(), Err(Error::InvalidBytes(vec![0xC0])));
        assert_eq!(iter.next(), Err(Error::End));
    }
}
