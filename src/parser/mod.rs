mod html_parser;
mod json_parser;

use std::error::Error;
use std::io::BufReader;
use std::io::Read;

use crate::model::Models;
use crate::utf8_reader::Utf8Iter;

pub use html_parser::ParseError;

pub enum ParserType {
    Html,
    Json,
}

pub fn parse_file(file_path: &str, parser_type: ParserType) -> Result<Models, Box<dyn Error>> {
    let file = std::fs::File::open(file_path)?;
    parser(parser_type, BufReader::new(file))
}

pub fn parser<F: Read>(parser_type: ParserType, data: F) -> Result<Models, Box<dyn Error>> {
    match parser_type {
        ParserType::Html => {
            let mut parser = html_parser::ModelsParser::new();
            match parser.parse(Utf8Iter::new(data)) {
                Ok(()) => Ok(parser.to_models()),
                Err(error) => Err(error.into()),
            }
        }
        ParserType::Json => json_parser::parse(data).map_err(|e| e.into()),
    }
}
