// ldraw-rust: LDraw models for Rust
// Copyright (C) 2017-2025  Edgar Gonzàlez i Pellicer
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

mod base;
mod meta;
mod primitives;

use na::convert_unchecked;
use na::core::dimension::U4;
use na::core::{ArrayStorage, Matrix as NAMatrix};
use std::fs::File;
use std::io::{BufRead, BufReader, Error as IoError};
use std::path::{Path, PathBuf};

use super::file::*;
use super::types::*;

use self::base::ParseResult1;

pub use self::base::{ParseError, ParseResult};

/// Loads an LDraw file from a stream.
pub fn load_ldraw(path: &Path) -> ParseResult<LDFile> {
    // Open the file and create a reader for it.
    let file = File::open(&path).map_err(|err: IoError| {
        vec![ParseError {
            path: path.to_path_buf(),
            line_number: 0,
            error: format!("Can't open {} for reading: {}", path.display(), err),
        }]
    })?;
    let mut reader = BufReader::new(file);

    // Resulting sets of statements and errors.
    let mut statements = Vec::new();
    let mut errors = Vec::new();

    // Read and process line by line.
    let mut current_line_number = 1;
    let mut current_line = String::new();
    let mut current_result = reader.read_line(&mut current_line);
    while base::read_line_is_ok(&current_result) {
        current_line = current_line
            .chars()
            .take_while(|c| *c != '\r' && *c != '\n')
            .collect();
        match process_line(path, current_line_number, &current_line) {
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
            path: path.to_path_buf(),
            line_number: current_line_number,
            error: format!("Can't read line: {}", err),
        })
    }

    // Return success if no errors were found, failure otherwise.
    if errors.is_empty() {
        Ok(LDFile {
            statements: statements,
        })
    } else {
        Err(errors)
    }
}

/// Processes a line of an LDraw file.
fn process_line(path: &Path, line_number: u32, line: &String) -> ParseResult1<Option<Statement>> {
    // Split the line into fields.
    let fields: Vec<&str> = line
        .split(|c| c == ' ' || c == '\t')
        .filter(|f| !f.is_empty())
        .collect();
    if fields.is_empty() {
        return Ok(None);
    }

    // Dispatch according to the line type.
    let line_type = base::parse_u8(path, line_number, fields[0])?;
    match line_type {
        0 => meta::parse_meta_statement(path, line_number, fields),
        1 => parse_subfile_statement(path, line_number, fields),
        2 => primitives::parse_line_statement(path, line_number, fields),
        3 => primitives::parse_triangle_statement(path, line_number, fields),
        4 => primitives::parse_quad_statement(path, line_number, fields),
        5 => primitives::parse_optional_line_statement(path, line_number, fields),
        _ => Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Invalid line type {}", line_type),
        }),
    }
    .map(Some)
}

/// Parse a line containing a Subfile statement.
fn parse_subfile_statement(
    path: &Path,
    line_number: u32,
    fields: Vec<&str>,
) -> ParseResult1<Statement> {
    type RawMatrix = NAMatrix<f64, U4, U4, ArrayStorage<f64, U4, U4>>;
    base::check_fields_eq(path, line_number, &fields, 15)?;
    let color_ref = base::parse_color_ref(path, line_number, fields[1])?;
    let origin = base::map_result_monad(|f| base::parse_f64(path, line_number, f), &fields[2..5])?;
    let transform =
        base::map_result_monad(|f| base::parse_f64(path, line_number, f), &fields[5..14])?;
    let raw_matrix = RawMatrix::new(
        transform[0],
        transform[1],
        transform[2],
        origin[0],
        transform[3],
        transform[4],
        transform[5],
        origin[1],
        transform[6],
        transform[7],
        transform[8],
        origin[2],
        0.0,
        0.0,
        0.0,
        1.0,
    );
    let matrix: Matrix = unsafe { convert_unchecked(raw_matrix) };
    Ok(Statement::Subfile {
        color: color_ref,
        matrix: matrix,
        file: PathBuf::from(fields[14].replace('\\', "/").to_lowercase()),
    })
}

// Local Variables:
// coding: utf-8
// End:
