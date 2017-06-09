// ldraw-rust: LDraw models for Rust
// Copyright (C) 2017  Edgar Gonzàlez i Pellicer
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

//! Loading of LDraw files.

use na::convert_unchecked;
use na::core::{Matrix as NAMatrix, MatrixArray};
use na::core::dimension::U4;
use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader, Error as IoError, Result as IoResult};
use std::num::{ParseFloatError, ParseIntError};
use std::path::Path;
use std::str::FromStr;

use super::file::*;
use super::types::*;

/// Parse error.
pub struct ParseError {
    pub line_number: u32,
    pub error: String,
}

/// List of parse errors.
pub type ParseErrors = Vec<ParseError>;

/// Result of a parsing operation.
pub type ParseResult<T> = Result<T, ParseErrors>;

/// Internal version of `ParseResult` when at most 1 error is expected.
type ParseResult1<T> = Result<T, ParseError>;

/// Loads an LDraw file from a stream.
pub fn load_ldraw<P: AsRef<Path> + Display>(path: P) -> ParseResult<LDFile> {
    // Open the file and create a reader for it.
    let file = try!(File::open(&path).map_err(|err: IoError| {
        vec![ParseError {
                 line_number: 0,
                 error: format!("Can't open {} for reading: {}", path, err),
             }]
    }));
    let mut reader = BufReader::new(file);

    // Resulting sets of statements and errors.
    let mut statements = Vec::new();
    let mut errors = Vec::new();

    // Read and process line by line.
    let mut current_line_number = 1;
    let mut current_line = String::new();
    let mut current_result = reader.read_line(&mut current_line);
    while read_line_is_ok(&current_result) {
        current_line = current_line.chars()
            .take_while(|c| *c != '\r' && *c != '\n')
            .collect();
        match process_line(current_line_number, &current_line) {
            Ok(None) => (),
            Ok(Some(statement)) => statements.push(statement),
            Err(err) => errors.push(err),
        }
        current_line_number += 1;
        current_line.clear();
        current_result = reader.read_line(&mut current_line);
    }

    // Append an error if we exited the loop because of a failure in
    // `read_line`.
    if let Err(err) = current_result {
        errors.push(ParseError {
            line_number: current_line_number,
            error: format!("Can't read line: {}", err),
        })
    }

    // Return success if no errors were found, failure otherwise.
    if errors.is_empty() {
        Ok(LDFile { statements: statements })
    } else {
        Err(errors)
    }
}

/// Returns true if the result of a `read_line` operation succeeded.
fn read_line_is_ok(result: &IoResult<usize>) -> bool {
    match *result {
        Ok(count) => count > 0,
        Err(_) => false,
    }
}

/// Processes a line of an LDraw file.
fn process_line(line_number: u32, line: &String)
    -> ParseResult1<Option<Statement>> {
    // Split the line into fields.
    let fields: Vec<&str> = line.split(|c| c == ' ' || c == '\t')
        .filter(|f| !f.is_empty())
        .collect();
    if fields.is_empty() {
        return Ok(None);
    }

    // Determine the line type.
    let line_type = try!(parse_u8(line_number, fields[0]));

    // Dispatch according to the line type.
    match line_type {
        0 => parse_meta_statement(line_number, fields),
        1 => parse_subfile_statement(line_number, fields),
        2 => parse_line_statement(line_number, fields),
        3 => parse_triangle_statement(line_number, fields),
        4 => parse_quad_statement(line_number, fields),
        5 => parse_optional_line_statement(line_number, fields),
        _ => {
            Err(ParseError {
                line_number: line_number,
                error: format!("Invalid line type {}", line_type),
            })
        },
    }
}

/// Parse a line containing a Meta statement.
fn parse_meta_statement(line_number: u32, fields: Vec<&str>)
    -> ParseResult1<Option<Statement>> {
    Ok(None)
}

/// Parse a line containing a Subfile statement.
#[rustfmt_skip]
fn parse_subfile_statement(line_number: u32, fields: Vec<&str>)
    -> ParseResult1<Option<Statement>> {
    type RawMatrix = NAMatrix<f64, U4, U4, MatrixArray<f64, U4, U4>>;
    if fields.len() != 15 {
        return Err(ParseError {
            line_number: line_number,
            error: format!("Line has {} fields: expected 15", fields.len()),
        });
    }
    let color_ref = try!(parse_color_ref(line_number, fields[1]));
    let origin = try!(map_result_monad(|f| parse_f64(line_number, f),
                                       &fields[2..5]));
    let transform = try!(map_result_monad(|f| parse_f64(line_number, f),
                                          &fields[5..14]));
    let raw_matrix = RawMatrix::new(
        transform[0], transform[1], transform[2], origin[0],
        transform[3], transform[4], transform[5], origin[1],
        transform[6], transform[7], transform[8], origin[2],
        0.0,          0.0,          0.0,          1.0);
    let matrix: Matrix = unsafe { convert_unchecked(raw_matrix) };
    Ok(Some(Statement::Subfile {
        color: color_ref,
        matrix: matrix,
        file: String::from(fields[14]),
    }))
}

/// Parse a line containing a Line statement.
fn parse_line_statement(line_number: u32, fields: Vec<&str>)
    -> ParseResult1<Option<Statement>> {
    if fields.len() != 8 {
        return Err(ParseError {
            line_number: line_number,
            error: format!("Line has {} fields: expected 8", fields.len()),
        });
    }
    let color_ref = try!(parse_color_ref(line_number, fields[1]));
    let coordinates = try!(map_result_monad(|f| parse_f64(line_number, f),
                                            &fields[2..8]));
    let a = Point::new(coordinates[0], coordinates[1], coordinates[2]);
    let b = Point::new(coordinates[3], coordinates[4], coordinates[5]);
    Ok(Some(Statement::Line { color: color_ref, line: Line { a: a, b: b } }))
}

/// Parse a line containing a Triangle statement.
fn parse_triangle_statement(line_number: u32, fields: Vec<&str>)
    -> ParseResult1<Option<Statement>> {
    if fields.len() != 11 {
        return Err(ParseError {
            line_number: line_number,
            error: format!("Line has {} fields: expected 11", fields.len()),
        });
    }
    let color_ref = try!(parse_color_ref(line_number, fields[1]));
    let coordinates = try!(map_result_monad(|f| parse_f64(line_number, f),
                                            &fields[2..11]));
    let a = Point::new(coordinates[0], coordinates[1], coordinates[2]);
    let b = Point::new(coordinates[3], coordinates[4], coordinates[5]);
    let c = Point::new(coordinates[6], coordinates[7], coordinates[8]);
    Ok(Some(Statement::Triangle {
        color: color_ref,
        triangle: Triangle { a: a, b: b, c: c },
    }))
}

/// Parse a line containing a Quad statement.
fn parse_quad_statement(line_number: u32, fields: Vec<&str>)
    -> ParseResult1<Option<Statement>> {
    if fields.len() != 14 {
        return Err(ParseError {
            line_number: line_number,
            error: format!("Line has {} fields: expected 14", fields.len()),
        });
    }
    let color_ref = try!(parse_color_ref(line_number, fields[1]));
    let coordinates = try!(map_result_monad(|f| parse_f64(line_number, f),
                                            &fields[2..14]));
    let a = Point::new(coordinates[0], coordinates[1], coordinates[2]);
    let b = Point::new(coordinates[3], coordinates[4], coordinates[5]);
    let c = Point::new(coordinates[6], coordinates[7], coordinates[8]);
    let d = Point::new(coordinates[9], coordinates[10], coordinates[11]);
    Ok(Some(Statement::Quad {
        color: color_ref,
        quad: Quad { a: a, b: b, c: c, d: d },
    }))
}

/// Parse a line containing an OptionalLine statement.
fn parse_optional_line_statement(line_number: u32, fields: Vec<&str>)
    -> ParseResult1<Option<Statement>> {
    if fields.len() != 14 {
        return Err(ParseError {
            line_number: line_number,
            error: format!("Line has {} fields: expected 14", fields.len()),
        });
    }
    let color_ref = try!(parse_color_ref(line_number, fields[1]));
    let coordinates = try!(map_result_monad(|f| parse_f64(line_number, f),
                                            &fields[2..14]));
    let a = Point::new(coordinates[0], coordinates[1], coordinates[2]);
    let b = Point::new(coordinates[3], coordinates[4], coordinates[5]);
    let c_a = Point::new(coordinates[6], coordinates[7], coordinates[8]);
    let c_b = Point::new(coordinates[9], coordinates[10], coordinates[11]);
    Ok(Some(Statement::OptionalLine {
        color: color_ref,
        line: Line { a: a, b: b },
        control_line: Line { a: c_a, b: c_b },
    }))
}

/// Parse an 8-bit integer.
fn parse_u8(line_number: u32, field: &str) -> ParseResult1<u8> {
    u8::from_str_radix(field, 10).map_err(|err: ParseIntError| {
        ParseError {
            line_number: line_number,
            error: format!("Can't parse \"{}\" as integer: {}", field, err),
        }
    })
}

/// Parse a 16-bit integer.
fn parse_u16(line_number: u32, field: &str) -> ParseResult1<u16> {
    u16::from_str_radix(field, 10).map_err(|err: ParseIntError| {
        ParseError {
            line_number: line_number,
            error: format!("Can't parse \"{}\" as integer: {}", field, err),
        }
    })
}

/// Parse a 64-bit float.
fn parse_f64(line_number: u32, field: &str) -> ParseResult1<f64> {
    f64::from_str(field).map_err(|err: ParseFloatError| {
        ParseError {
            line_number: line_number,
            error: format!("Can't parse \"{}\" as float: {}", field, err),
        }
    })
}

/// Parse a field containing a color reference.
///
/// TODO: Add support for RGB colors.
fn parse_color_ref(line_number: u32, field: &str) -> ParseResult1<ColorRef> {
    if let Ok(index) = u16::from_str_radix(field, 10) {
        return Ok(ColorRef::Indexed(index));
    }
    Err(ParseError {
        line_number: line_number,
        error: format!("Can't parse \"{}\" as color", field),
    })
}

/// Map a function over a container on the Result monad.
fn map_result_monad<F, I, O, E>(func: F, inputs: I) -> Result<Vec<O>, E>
    where I: IntoIterator,
          F: Fn(I::Item) -> Result<O, E> {
    let mut vec = Vec::new();
    for input in inputs {
        vec.push(try!(func(input)));
    }
    Ok(vec)
}

// Local Variables:
// coding: utf-8
// End:
