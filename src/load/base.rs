// ldraw-rust: LDraw models for Rust
// Copyright (C) 2017-2019  Edgar Gonzàlez i Pellicer
//
// This file is part of ldraw-rust.
//
// ldraw-rust is free software: you can redistribute it and/or
// modify it under the terms of the GNU General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Loading of LDraw files: Base operations.

use regex::Regex;
// use std::f64;
use std::io::Result as IoResult;
use std::num::{ParseFloatError, ParseIntError};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::super::types::{ColorRef, RGB};

/// Parse error.
#[derive(Debug)]
pub struct ParseError {
    pub path: PathBuf,
    pub line_number: u32,
    pub error: String,
}

/// Result of a parsing operation.
pub type ParseResult<T> = Result<T, Vec<ParseError>>;

/// Internal version of `ParseResult` when at most 1 error is expected.
pub type ParseResult1<T> = Result<T, ParseError>;

/// Returns true if the result of a `read_line` operation succeeded.
pub fn read_line_is_ok(result: &IoResult<usize>) -> bool {
    match *result {
        Ok(count) => count > 0,
        Err(_) => false,
    }
}

/// Maps a function over a container on the Result monad.
pub fn map_result_monad<F, I, O, E>(func: F, inputs: I) -> Result<Vec<O>, E>
where
    I: IntoIterator,
    F: Fn(I::Item) -> Result<O, E>,
{
    let mut vec = Vec::new();
    for input in inputs {
        vec.push(func(input)?);
    }
    Ok(vec)
}

/// Verifies that a field in a line matches a keyword'.
pub fn check_field_is(
    path: &Path,
    line_number: u32,
    field: &str,
    keyword: &str,
) -> ParseResult1<()> {
    if field != keyword {
        Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!(
                "Found \"{}\" as field where \"{}\" was expected",
                field, keyword
            ),
        })
    } else {
        Ok(())
    }
}

/// Verifies that the number of fields in a line is exactly 'n'.
pub fn check_fields_eq(
    path: &Path,
    line_number: u32,
    fields: &Vec<&str>,
    n: usize,
) -> ParseResult1<()> {
    if fields.len() != n {
        Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Line has {} fields: expected {}", fields.len(), n),
        })
    } else {
        Ok(())
    }
}

/// Verifies that the number of fields in a line is at least 'n'.
pub fn check_fields_ge(
    path: &Path,
    line_number: u32,
    fields: &Vec<&str>,
    n: usize,
) -> ParseResult1<()> {
    if fields.len() < n {
        Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Line has {} fields: expected at least {}", fields.len(), n),
        })
    } else {
        Ok(())
    }
}

/// Verifies that the number of fields in a line is at most 'n'.
pub fn check_fields_le(
    path: &Path,
    line_number: u32,
    fields: &Vec<&str>,
    n: usize,
) -> ParseResult1<()> {
    if fields.len() > n {
        Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Line has {} fields: expected at most {}", fields.len(), n),
        })
    } else {
        Ok(())
    }
}

/// Verifies that a field in a line exists and returns it.
pub fn check_field_get<'a>(
    path: &Path,
    line_number: u32,
    fields: &Vec<&'a str>,
    index: usize,
) -> ParseResult1<&'a str> {
    if fields.len() < index {
        Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!(
                "Line has {} fields: expected at least {}",
                fields.len(),
                index
            ),
        })
    } else {
        Ok(fields[index])
    }
}

/// Verifies that a field in a line exists and can be parsed.
pub fn check_field_get_and<F, O>(
    path: &Path,
    line_number: u32,
    fields: &Vec<&str>,
    index: usize,
    parser: F,
) -> ParseResult1<O>
where
    F: Fn(&Path, u32, &str) -> ParseResult1<O>,
{
    if fields.len() < index {
        Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!(
                "Line has {} fields: expected at least {}",
                fields.len(),
                index
            ),
        })
    } else {
        parser(path, line_number, fields[index])
    }
}

/// Parses an 8-bit integer.
pub fn parse_u8(path: &Path, line_number: u32, field: &str) -> ParseResult1<u8> {
    u8::from_str_radix(field, 10).map_err(|err: ParseIntError| ParseError {
        path: path.to_path_buf(),
        line_number: line_number,
        error: format!("Can't parse \"{}\" as integer: {}", field, err),
    })
}

/// Parses an 16-bit integer.
pub fn parse_u16(path: &Path, line_number: u32, field: &str) -> ParseResult1<u16> {
    u16::from_str_radix(field, 10).map_err(|err: ParseIntError| ParseError {
        path: path.to_path_buf(),
        line_number: line_number,
        error: format!("Can't parse \"{}\" as integer: {}", field, err),
    })
}

/// Parses a 64-bit float.
pub fn parse_f64(path: &Path, line_number: u32, field: &str) -> ParseResult1<f64> {
    f64::from_str(field).map_err(|err: ParseFloatError| ParseError {
        path: path.to_path_buf(),
        line_number: line_number,
        error: format!("Can't parse \"{}\" as float: {}", field, err),
    })
}

/// Parses an RGB color.
pub fn parse_rgb(path: &Path, line_number: u32, field: &str) -> ParseResult1<RGB> {
    lazy_static! {
        static ref RGB_RE: Regex =
            Regex::new(r"^(?:#|0x2)([[:xdigit:]]{2})([[:xdigit:]]{2})([[:xdigit:]]{2})$").unwrap();
    }
    let captures = match RGB_RE.captures(&field) {
        None => {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Can't parse \"{}\" as RGB color", field),
            });
        }
        Some(captures) => captures,
    };
    let red = u8::from_str_radix(captures.get(1).unwrap().as_str(), 16).unwrap();
    let green = u8::from_str_radix(captures.get(2).unwrap().as_str(), 16).unwrap();
    let blue = u8::from_str_radix(captures.get(3).unwrap().as_str(), 16).unwrap();
    Ok(RGB(red, green, blue))
}

/// Parses a color reference.
pub fn parse_color_ref(path: &Path, line_number: u32, field: &str) -> ParseResult1<ColorRef> {
    if let Ok(index) = u16::from_str_radix(field, 10) {
        return Ok(ColorRef::Indexed(index));
    }
    if let Ok(rgb) = parse_rgb(path, line_number, field) {
        return Ok(ColorRef::RGB(rgb));
    }
    Err(ParseError {
        path: path.to_path_buf(),
        line_number: line_number,
        error: format!("Can't parse \"{}\" as color reference", field),
    })
}

// Local Variables:
// coding: utf-8
// End:
