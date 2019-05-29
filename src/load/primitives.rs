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

//! Loading of LDraw files: Primitives.

use std::path::Path;

use super::super::file::Statement;
use super::super::types::{Line, Point, Quad, Triangle};

use super::base::{self, ParseResult1};

/// Parse a line containing a Line statement.
pub fn parse_line_statement(
    path: &Path,
    line_number: u32,
    fields: Vec<&str>,
) -> ParseResult1<Statement> {
    base::check_fields_eq(path, line_number, &fields, 8)?;
    let color_ref = base::parse_color_ref(path, line_number, fields[1])?;
    let coordinates =
        base::map_result_monad(|f| base::parse_f64(path, line_number, f), &fields[2..8])?;
    let a = Point::new(coordinates[0], coordinates[1], coordinates[2]);
    let b = Point::new(coordinates[3], coordinates[4], coordinates[5]);
    Ok(Statement::Line {
        color: color_ref,
        line: Line { a: a, b: b },
    })
}

/// Parse a line containing a Triangle statement.
pub fn parse_triangle_statement(
    path: &Path,
    line_number: u32,
    fields: Vec<&str>,
) -> ParseResult1<Statement> {
    base::check_fields_eq(path, line_number, &fields, 11)?;
    let color_ref = base::parse_color_ref(path, line_number, fields[1])?;
    let coordinates =
        base::map_result_monad(|f| base::parse_f64(path, line_number, f), &fields[2..11])?;
    let a = Point::new(coordinates[0], coordinates[1], coordinates[2]);
    let b = Point::new(coordinates[3], coordinates[4], coordinates[5]);
    let c = Point::new(coordinates[6], coordinates[7], coordinates[8]);
    Ok(Statement::Triangle {
        color: color_ref,
        triangle: Triangle { a: a, b: b, c: c },
    })
}

/// Parse a line containing a Quad statement.
pub fn parse_quad_statement(
    path: &Path,
    line_number: u32,
    fields: Vec<&str>,
) -> ParseResult1<Statement> {
    base::check_fields_eq(path, line_number, &fields, 14)?;
    let color_ref = base::parse_color_ref(path, line_number, fields[1])?;
    let coordinates =
        base::map_result_monad(|f| base::parse_f64(path, line_number, f), &fields[2..14])?;
    let a = Point::new(coordinates[0], coordinates[1], coordinates[2]);
    let b = Point::new(coordinates[3], coordinates[4], coordinates[5]);
    let c = Point::new(coordinates[6], coordinates[7], coordinates[8]);
    let d = Point::new(coordinates[9], coordinates[10], coordinates[11]);
    Ok(Statement::Quad {
        color: color_ref,
        quad: Quad {
            a: a,
            b: b,
            c: c,
            d: d,
        },
    })
}

/// Parse a line containing an OptionalLine statement.
pub fn parse_optional_line_statement(
    path: &Path,
    line_number: u32,
    fields: Vec<&str>,
) -> ParseResult1<Statement> {
    base::check_fields_eq(path, line_number, &fields, 14)?;
    let color_ref = base::parse_color_ref(path, line_number, fields[1])?;
    let coordinates =
        base::map_result_monad(|f| base::parse_f64(path, line_number, f), &fields[2..14])?;
    let a = Point::new(coordinates[0], coordinates[1], coordinates[2]);
    let b = Point::new(coordinates[3], coordinates[4], coordinates[5]);
    let c_a = Point::new(coordinates[6], coordinates[7], coordinates[8]);
    let c_b = Point::new(coordinates[9], coordinates[10], coordinates[11]);
    Ok(Statement::OptionalLine {
        color: color_ref,
        line: Line { a: a, b: b },
        control_line: Line { a: c_a, b: c_b },
    })
}

// Local Variables:
// coding: utf-8
// End:
