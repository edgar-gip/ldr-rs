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
use na::core::{ArrayStorage, Matrix as NAMatrix};
use na::core::dimension::U4;
use phf::{self, phf_map};
use regex::Regex;
use std::f64;
use std::fs::File;
use std::io::{BufRead, BufReader, Error as IoError, Result as IoResult};
use std::num::{ParseFloatError, ParseIntError};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::file::*;
use super::types::*;

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
type ParseResult1<T> = Result<T, ParseError>;

/// Loads an LDraw file from a stream.
pub fn load_ldraw(path: &Path) -> ParseResult<LDFile> {
    // Open the file and create a reader for it.
    let file = File::open(&path).map_err(|err: IoError| {
        vec![ParseError {
                 path: path.to_path_buf(),
                 line_number: 0,
                 error: format!("Can't open {} for reading: {}",
                                path.display(),
                                err),
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
    while read_line_is_ok(&current_result) {
        current_line = current_line.chars()
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
fn process_line(path: &Path,
                line_number: u32,
                line: &String)
    -> ParseResult1<Option<Statement>> {
    // Split the line into fields.
    let fields: Vec<&str> = line.split(|c| c == ' ' || c == '\t')
        .filter(|f| !f.is_empty())
        .collect();
    if fields.is_empty() {
        return Ok(None);
    }

    // Dispatch according to the line type.
    let line_type = parse_u8(path, line_number, fields[0])?;
    match line_type {
            0 => parse_meta_statement(path, line_number, fields),
            1 => parse_subfile_statement(path, line_number, fields),
            2 => parse_line_statement(path, line_number, fields),
            3 => parse_triangle_statement(path, line_number, fields),
            4 => parse_quad_statement(path, line_number, fields),
            5 => parse_optional_line_statement(path, line_number, fields),
            _ => {
                Err(ParseError {
                    path: path.to_path_buf(),
                    line_number: line_number,
                    error: format!("Invalid line type {}", line_type),
                })
            },
        }
        .map(Some)
}

/// Maps a function over a container on the Result monad.
fn map_result_monad<F, I, O, E>(func: F, inputs: I) -> Result<Vec<O>, E>
    where I: IntoIterator,
          F: Fn(I::Item) -> Result<O, E> {
    let mut vec = Vec::new();
    for input in inputs {
        vec.push(func(input)?);
    }
    Ok(vec)
}

/// Verifies that a field in a line matches a keyword'.
fn check_field_is(path: &Path,
                  line_number: u32,
                  field: &str,
                  keyword: &str)
    -> ParseResult1<()> {
    if field != keyword {
        Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Found \"{}\" as field where \"{}\" was expected",
                           field,
                           keyword),
        })
    } else {
        Ok(())
    }
}

/// Verifies that the number of fields in a line is exactly 'n'.
fn check_fields_eq(path: &Path,
                   line_number: u32,
                   fields: &Vec<&str>,
                   n: usize)
    -> ParseResult1<()> {
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
fn check_fields_ge(path: &Path,
                   line_number: u32,
                   fields: &Vec<&str>,
                   n: usize)
    -> ParseResult1<()> {
    if fields.len() < n {
        Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Line has {} fields: expected at least {}",
                           fields.len(),
                           n),
        })
    } else {
        Ok(())
    }
}

/// Verifies that the number of fields in a line is at most 'n'.
fn check_fields_le(path: &Path,
                   line_number: u32,
                   fields: &Vec<&str>,
                   n: usize)
    -> ParseResult1<()> {
    if fields.len() > n {
        Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Line has {} fields: expected at most {}",
                           fields.len(),
                           n),
        })
    } else {
        Ok(())
    }
}

/// Verifies that a field in a line exists and returns it.
fn check_field_get<'a>(path: &Path,
                       line_number: u32,
                       fields: &Vec<&'a str>,
                       index: usize)
    -> ParseResult1<&'a str> {
    if fields.len() < index {
        Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Line has {} fields: expected at least {}",
                           fields.len(),
                           index),
        })
    } else {
        Ok(fields[index])
    }
}

/// Verifies that a field in a line exists and can be parsed.
fn check_field_get_and<F, O>(path: &Path,
                             line_number: u32,
                             fields: &Vec<&str>,
                             index: usize,
                             parser: F)
    -> ParseResult1<O>
    where F: Fn(&Path, u32, &str) -> ParseResult1<O> {
    if fields.len() < index {
        Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Line has {} fields: expected at least {}",
                           fields.len(),
                           index),
        })
    } else {
        parser(path, line_number, fields[index])
    }
}

/// Parses an 8-bit integer.
fn parse_u8(path: &Path, line_number: u32, field: &str) -> ParseResult1<u8> {
    u8::from_str_radix(field, 10).map_err(|err: ParseIntError| {
        ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Can't parse \"{}\" as integer: {}", field, err),
        }
    })
}

/// Parses an 16-bit integer.
fn parse_u16(path: &Path, line_number: u32, field: &str) -> ParseResult1<u16> {
    u16::from_str_radix(field, 10).map_err(|err: ParseIntError| {
        ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Can't parse \"{}\" as integer: {}", field, err),
        }
    })
}

/// Parses a 64-bit float.
fn parse_f64(path: &Path, line_number: u32, field: &str) -> ParseResult1<f64> {
    f64::from_str(field).map_err(|err: ParseFloatError| {
        ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Can't parse \"{}\" as float: {}", field, err),
        }
    })
}

/// Parses an RGB color.
fn parse_rgb(path: &Path, line_number: u32, field: &str) -> ParseResult1<RGB> {
    lazy_static! {
        static ref RGB_RE: Regex = Regex::new(
            r"^(?:#|0x2)([[:xdigit:]]{2})([[:xdigit:]]{2})([[:xdigit:]]{2})$")
            .unwrap();
    }
    let captures = match RGB_RE.captures(&field) {
        None => {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Can't parse \"{}\" as RGB color", field),
            });
        },
        Some(captures) => captures,
    };
    let red = u8::from_str_radix(captures.get(1).unwrap().as_str(), 16)
        .unwrap();
    let green = u8::from_str_radix(captures.get(2).unwrap().as_str(), 16)
        .unwrap();
    let blue = u8::from_str_radix(captures.get(3).unwrap().as_str(), 16)
        .unwrap();
    Ok(RGB(red, green, blue))
}

/// Parses a color reference.
fn parse_color_ref(path: &Path,
                   line_number: u32,
                   field: &str)
    -> ParseResult1<ColorRef> {
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

/// Parses an update tag.
fn parse_update_tag(path: &Path,
                    line_number: u32,
                    field: &str)
    -> ParseResult1<UpdateTag> {
    lazy_static! {
        static ref TAG_RE: Regex = Regex::new(
            r"^(\d{4})-(\d{2}|\?\?)(?:-(\d{2}|\?\?))?")
            .unwrap();
    }
    let captures = match TAG_RE.captures(field) {
        None => {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Can't parse \"{}\" as an update tag", field),
            });
        },
        Some(captures) => captures,
    };

    let year = u16::from_str(captures.get(1).unwrap().as_str()).unwrap();
    let month_str = captures.get(2).unwrap().as_str();
    let month = if month_str == "??" {
        None
    } else {
        Some(u8::from_str(month_str).unwrap())
    };
    match captures.get(3) {
        Some(day_match) => {
            let day_str = day_match.as_str();
            let day = if day_str == "??" {
                None
            } else {
                Some(u8::from_str(day_str).unwrap())
            };
            Ok(UpdateTag::Date(Date { year: year, month: month, day: day }))
        },
        None => {
            match month {
                Some(release) => {
                    Ok(UpdateTag::Release(Release {
                        year: year,
                        release: release,
                    }))
                },
                None => {
                    Err(ParseError {
                        path: path.to_path_buf(),
                        line_number: line_number,
                        error: format!("Can't use \"??\" as a release"),
                    })
                },
            }
        },
    }
}

/// Parses a line containing an argument-less meta statement.
fn parse_meta_argumentless(path: &Path,
                           line_number: u32,
                           fields: Vec<&str>,
                           meta: Meta)
    -> ParseResult1<Meta> {
    check_fields_eq(path, line_number, &fields, 2)?;
    Ok(meta)
}

/// Parses a line containing a full-line meta statement.
fn parse_meta_full_line(path: &Path,
                        line_number: u32,
                        fields: Vec<&str>,
                        meta_constructor: fn(String) -> Meta)
    -> ParseResult1<Meta> {
    check_fields_ge(path, line_number, &fields, 2)?;
    Ok(meta_constructor(fields[2..fields.len()].join(" ")))
}

/// Parse a line containing an Author Meta statement.
fn parse_meta_author(path: &Path,
                     line_number: u32,
                     fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_full_line(path, line_number, fields, Meta::Author)
}

/// Parse a line containing an BFC Meta statement.
fn parse_meta_bfc(path: &Path,
                  line_number: u32,
                  fields: Vec<&str>)
    -> ParseResult1<Meta> {
    check_fields_ge(path, line_number, &fields, 3)?;
    let bfc = if fields[2] == "NOCERTIFY" {
        check_fields_eq(path, line_number, &fields, 3)?;
        BFCDeclaration::NoCertify
    } else if fields[2] == "CERTIFY" {
        check_fields_le(path, line_number, &fields, 4)?;
        if fields.len() == 3 || fields[3] == "CCW" {
            BFCDeclaration::Certify(RotationSense::CCW)
        } else if fields[3] == "CW" {
            BFCDeclaration::Certify(RotationSense::CW)
        } else {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Can't parse \"{}\" as rotation sense",
                               fields[3]),
            });
        }
    } else if fields[2] == "CW" {
        check_fields_le(path, line_number, &fields, 4)?;
        if fields.len() == 3 {
            BFCDeclaration::Rotation(RotationSense::CW)
        } else if fields[3] == "CLIP" {
            BFCDeclaration::Clip(Some(RotationSense::CW))
        } else {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Can't parse \"{}\" as BFC declaration",
                               fields[2..fields.len()].join(" ")),
            });
        }
    } else if fields[2] == "CCW" {
        check_fields_le(path, line_number, &fields, 4)?;
        if fields.len() == 3 {
            BFCDeclaration::Rotation(RotationSense::CCW)
        } else if fields[3] == "CLIP" {
            BFCDeclaration::Clip(Some(RotationSense::CCW))
        } else {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Can't parse \"{}\" as BFC declaration",
                               fields[2..fields.len()].join(" ")),
            });
        }
    } else if fields[2] == "CLIP" {
        check_fields_le(path, line_number, &fields, 4)?;
        if fields.len() == 3 {
            BFCDeclaration::Clip(None)
        } else if fields[3] == "CCW" {
            BFCDeclaration::Clip(Some(RotationSense::CCW))
        } else if fields[3] == "CW" {
            BFCDeclaration::Clip(Some(RotationSense::CW))
        } else {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Can't parse \"{}\" as rotation sense",
                               fields[3]),
            });
        }
    } else if fields[2] == "NOCLIP" {
        check_fields_eq(path, line_number, &fields, 3)?;
        BFCDeclaration::NoClip
    } else if fields[2] == "INVERTNEXT" {
        check_fields_eq(path, line_number, &fields, 3)?;
        BFCDeclaration::InvertNext
    } else {
        return Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Can't parse \"{}\" as BFC declaration",
                           fields[2..fields.len()].join(" ")),
        });
    };
    Ok(Meta::BFC(bfc))
}

/// Parse a line containing a !CATEGORY Meta statement.
fn parse_meta_category(path: &Path,
                       line_number: u32,
                       fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_full_line(path, line_number, fields, Meta::Category)
}

/// Parse a line containing a CLEAR Meta statement.
fn parse_meta_clear(path: &Path,
                    line_number: u32,
                    fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_argumentless(path, line_number, fields, Meta::Clear)
}

/// Parse a line containing a !CMDLINE Meta statement.
fn parse_meta_cmdline(path: &Path,
                      line_number: u32,
                      fields: Vec<&str>)
    -> ParseResult1<Meta> {
    check_fields_ge(path, line_number, &fields, 3)?;
    Ok(Meta::CmdLine(fields[2..fields.len()]
        .iter()
        .map(|s| s.to_string())
        .collect()))
}

/// Parse a MATERIAL declaration at the end of a !COLOUR Meta statement.
fn parse_color_material(path: &Path,
                        line_number: u32,
                        fields: &Vec<&str>,
                        index: &mut usize)
    -> ParseResult1<Material> {
    if *index == fields.len() {
        return Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("MATERIAL keyword missing its arguments"),
        });
    }
    let start_index = *index;
    if fields[start_index] == "GLITTER" || fields[start_index] == "SPECKLE" {
        *index += 1;

        // Parse VALUE <value>.
        check_field_get_and(path,
                            line_number,
                            &fields,
                            *index,
                            |p, l, f| check_field_is(p, l, f, "VALUE"))?;
        let value = check_field_get_and(path,
                                        line_number,
                                        fields,
                                        *index + 1,
                                        &parse_rgb)?;
        *index += 2;

        // Optionally parse ALPHA <alpha>.
        let alpha = if *index == fields.len() || fields[*index] != "ALPHA" {
            None
        } else if *index == fields.len() - 1 {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("ALPHA keyword missing its argument"),
            });
        } else {
            *index += 2;
            Some(parse_u8(path, line_number, fields[*index - 1])?)
        };

        // Optionally parse LUMINANCE <luminance>.
        let luminance = if *index == fields.len() ||
                           fields[*index] != "LUMINANCE" {
            None
        } else if *index == fields.len() - 1 {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("LUMINANCE keyword missing its argument"),
            });
        } else {
            *index += 2;
            Some(parse_u8(path, line_number, fields[*index - 1])?)
        };

        // Parse FRACTION <fraction>.
        check_field_get_and(path,
                            line_number,
                            &fields,
                            *index,
                            |p, l, f| {
            check_field_is(p, l, f, "FRACTION")
        })?;
        let fraction = check_field_get_and(path,
                                           line_number,
                                           fields,
                                           *index + 1,
                                           &parse_f64)?;
        if fraction < 0.0 || fraction > 1.0 {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("FRACTION value {} outside of [0.0, 1.0] range",
                               fraction),
            });
        }
        *index += 2;

        // Parse VFRACTION <v_fraction> if the material is GLITTER.
        let v_fraction = if fields[start_index] == "GLITTER" {
            check_field_get_and(path,
                                line_number,
                                &fields,
                                *index,
                                |p, l, f| {
                check_field_is(p, l, f, "VFRACTION")
            })?;
            let v_fraction = check_field_get_and(path,
                                                 line_number,
                                                 fields,
                                                 *index + 1,
                                                 &parse_f64)?;
            if v_fraction < 0.0 || v_fraction > 1.0 {
                return Err(ParseError {
                    path: path.to_path_buf(),
                    line_number: line_number,
                    error: format!("VFRACTION value {} outside of [0.0, 1.0] \
                                    range",
                                   v_fraction),
                });
            }
            *index += 2;
            v_fraction
        } else {
            f64::NAN
        };

        // Parse SIZE <size> | MINSIZE <min_size> MAXSIZE <max_size>.
        let size_kw = check_field_get(path, line_number, &fields, *index)?;
        let size = if size_kw == "SIZE" {
            let size = check_field_get_and(path,
                                           line_number,
                                           fields,
                                           *index + 1,
                                           &parse_u8)?;
            *index += 2;
            (size, size)
        } else if size_kw == "MINSIZE" {
            let minsize = check_field_get_and(path,
                                              line_number,
                                              fields,
                                              *index + 1,
                                              &parse_u8)?;
            check_field_get_and(path,
                                line_number,
                                &fields,
                                *index + 2,
                                |p, l, f| {
                check_field_is(p, l, f, "MAXSIZE")
            })?;
            let maxsize = check_field_get_and(path,
                                              line_number,
                                              fields,
                                              *index + 3,
                                              &parse_u8)?;
            *index += 4;
            (minsize, maxsize)
        } else {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Unknown keyword \"{}\" in MATERIAL {}",
                               size_kw,
                               fields[start_index]),
            });
        };

        if fields[start_index] == "GLITTER" {
            Ok(Material::Glitter(Glitter {
                value: value,
                alpha: alpha,
                luminance: luminance,
                fraction: fraction,
                v_fraction: v_fraction,
                size: size,
            }))
        } else {
            //  fields[index] == "SPECKLE" {
            Ok(Material::Speckle(Speckle {
                value: value,
                alpha: alpha,
                luminance: luminance,
                fraction: fraction,
                size: size,
            }))
        }

    } else {
        let start_index = *index;
        *index = fields.len();
        Ok(Material::Unknown(fields[start_index..fields.len()]
            .iter()
            .map(|f| f.to_string())
            .collect()))
    }
}

/// Parse a line containing a !COLOUR Meta statement.
fn parse_meta_color(path: &Path,
                    line_number: u32,
                    fields: Vec<&str>)
    -> ParseResult1<Meta> {
    check_fields_ge(path, line_number, &fields, 9)?;

    // Parse <name> CODE <code> VALUE <value> EDGE <edge>.
    let name = fields[2].to_string();
    check_field_is(path, line_number, fields[3], "CODE")?;
    let code = parse_u16(path, line_number, fields[4])?;
    check_field_is(path, line_number, fields[5], "VALUE")?;
    let value = parse_rgb(path, line_number, fields[6])?;
    check_field_is(path, line_number, fields[7], "EDGE")?;
    let edge = parse_color_ref(path, line_number, fields[8])?;

    // Next unparsed field.
    let mut index: usize = 9;

    // Optionally parse ALPHA <alpha>.
    let alpha = if index == fields.len() || fields[index] != "ALPHA" {
        None
    } else if index == fields.len() - 1 {
        return Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("ALPHA keyword missing its argument"),
        });
    } else {
        index += 2;
        Some(parse_u8(path, line_number, fields[index - 1])?)
    };

    // Optionally parse LUMINANCE <luminance>.
    let luminance = if index == fields.len() || fields[index] != "LUMINANCE" {
        None
    } else if index == fields.len() - 1 {
        return Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("LUMINANCE keyword missing its argument"),
        });
    } else {
        index += 2;
        Some(parse_u8(path, line_number, fields[index - 1])?)
    };

    // Optionally parse finishes.
    let finish = if index == fields.len() {
        None
    } else if fields[index] == "CHROME" {
        index += 1;
        Some(Finish::Chrome)
    } else if fields[index] == "PEARLESCENT" {
        index += 1;
        Some(Finish::Pearlescent)
    } else if fields[index] == "RUBBER" {
        index += 1;
        Some(Finish::Rubber)
    } else if fields[index] == "MATE_METALLIC" {
        index += 1;
        Some(Finish::MatteMetallic)
    } else if fields[index] == "METAL" {
        index += 1;
        Some(Finish::Metal)
    } else if fields[index] == "MATERIAL" {
        index += 1;
        Some(Finish::Material(parse_color_material(path,
                                                   line_number,
                                                   &fields,
                                                   &mut index)?))
    } else {
        None
    };

    // Report unprocessed fields.
    if index < fields.len() {
        return Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Unprocessed \"{}\" at the end of !COLOUR",
                           fields[index..fields.len()].join(" ")),
        });
    }

    Ok(Meta::Color(Color {
        name: name,
        code: code,
        value: value,
        edge: edge,
        alpha: alpha,
        luminance: luminance,
        finish: finish,
    }))
}

/// Parse a line containing a comment Meta statement.
fn parse_meta_comment(path: &Path,
                      line_number: u32,
                      fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_full_line(path, line_number, fields, Meta::Comment)
}

/// Parse a line containing a FILE Meta statement.
fn parse_meta_file(path: &Path,
                   line_number: u32,
                   fields: Vec<&str>)
    -> ParseResult1<Meta> {
    check_fields_eq(path, line_number, &fields, 3)?;
    Ok(Meta::File(fields[2].to_string()))
}

/// Parse a line containing a file-type (!LDRAW_ORG) Meta statement.
fn parse_meta_file_type(path: &Path,
                        line_number: u32,
                        fields: Vec<&str>)
    -> ParseResult1<Meta> {
    static CONTENTS: phf::Map<&'static str, Contents> = phf_map! {
        "48_Primitive" => Contents::Primitive48,
        "8_Primitive" => Contents::Primitive8,
        "Alias" => Contents::Alias,
        "Configuration" => Contents::Configuration,
        "Cross-reference" => Contents::CrossReference,
        "Element" => Contents::Element,
        "File" => Contents::File,
        "Hi-Res Primitive" => Contents::HiResPrimitive,
        "Model" => Contents::Model,
        "Part" => Contents::Part,
        "Primitive" => Contents::Primitive,
        "Shortcut" => Contents::Shortcut,
        "Sub-part" => Contents::SubPart,
        "Submodel" => Contents::Submodel,
        "Subpart" => Contents::Subpart,
    };

    // Next unparsed field.
    let mut index;

    // Parse officiality.
    let mut officiality = if fields[1] == "!LDRAW_ORG" ||
                             fields[1] == "LDRAW_ORG" {
        index = 2;
        Officiality::LDrawOfficial
    } else if fields[1] == "Unofficial" ||
                                    fields[1] == "Un-official" {
        index = 2;
        Officiality::Unofficial
    } else {
        // fields[1] == "Official"
        check_field_get_and(path,
                            line_number,
                            &fields,
                            2,
                            |p, l, f| check_field_is(p, l, f, "LCAD"))?;
        index = 3;
        Officiality::LDrawOfficial
    };

    // Parse contents.
    let contents: Option<Contents> = if index == fields.len() {
        if officiality == Officiality::LDrawOfficial {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Missing fields: expected content description"),
            });
        }
        None
    } else if fields[index]
        .starts_with("Unofficial_") {
        officiality = Officiality::Unofficial;
        let contents_field = fields[index].trim_start_matches("Unofficial_");
        match CONTENTS.get(contents_field) {
            Some(contents) => Some(*contents),
            None => {
                return Err(ParseError {
                    path: path.to_path_buf(),
                    line_number: line_number,
                    error: format!("Can't parse \"{}\" as content type",
                                   fields[index]),
                })
            },
        }
    } else {
        match CONTENTS.get(fields[index]) {
            Some(contents) => Some(*contents),
            None => {
                return Err(ParseError {
                    path: path.to_path_buf(),
                    line_number: line_number,
                    error: format!("Can't parse \"{}\" as content type",
                                   fields[index]),
                })
            },
        }
    };
    index += 1;

    // Parse qualifiers.
    let mut qualifiers = Vec::new();
    loop {
        if index == fields.len() {
            break;
        }
        if fields[index] == "Alias" {
            qualifiers.push(Qualifier::Alias);
            index += 1;
        } else if fields[index] == "Physical_Colour" {
            qualifiers.push(Qualifier::PhysicalColor);
            index += 1;
        } else {
            break;
        }
    }

    // Parse update tag.
    let update_tag = if index == fields.len() {
        if officiality == Officiality::LDrawOfficial {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Missing fields: expected update tag"),
            });
        }
        None
    } else if fields[index] == "ORIGINAL" {
        index += 1;
        Some(UpdateTag::Original)
    } else if fields[index] == "UPDATE" {
        index += 1;
        if index == fields.len() {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("UPDATE keyword missing its argument"),
            });
        }
        index += 1;
        Some(parse_update_tag(path, line_number, fields[index - 1])?)
    } else {
        if officiality == Officiality::LDrawOfficial {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Can't parse \"{}\" as update tag",
                               fields[index]),
            });
        }
        None
    };

    // Report unprocessed fields.
    if officiality == Officiality::LDrawOfficial && index < fields.len() {
        return Err(ParseError {
            path: path.to_path_buf(),
            line_number: line_number,
            error: format!("Unprocessed \"{}\" at the end of {}",
                           fields[index..fields.len()].join(" "),
                           fields[1]),
        });
    }

    Ok(Meta::FileType(FileType {
        officiality: officiality,
        contents: contents,
        qualifiers: qualifiers,
        update_tag: update_tag,
    }))
}

/// Parse a line containing a !HELP Meta statement.
fn parse_meta_help(path: &Path,
                   line_number: u32,
                   fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_full_line(path, line_number, fields, Meta::Help)
}

/// Parse a line containing a !HISTORY Meta statement.
fn parse_meta_history(path: &Path,
                      line_number: u32,
                      fields: Vec<&str>)
    -> ParseResult1<Meta> {
    let normalized_line = fields[2..fields.len()].join(" ");
    lazy_static! {
        static ref LINE_RE: Regex = Regex::new(
            r"^(\d{4})-(\d{2}|\?\?)-(\d{2}|\?\?) (?:\[(.+)\]|\{(.+)\}) (.+)$")
            .unwrap();
    }
    let captures = match LINE_RE.captures(&normalized_line) {
        None => {
            return Err(ParseError {
                path: path.to_path_buf(),
                line_number: line_number,
                error: format!("Can't parse \"{}\" as history entry",
                               normalized_line),
            });
        },
        Some(captures) => captures,
    };

    let year = u16::from_str(captures.get(1).unwrap().as_str()).unwrap();
    let month_str = captures.get(2).unwrap().as_str();
    let month = if month_str == "??" {
        None
    } else {
        Some(u8::from_str(month_str).unwrap())
    };
    let day_str = captures.get(3).unwrap().as_str();
    let day = if day_str == "??" {
        None
    } else {
        Some(u8::from_str(day_str).unwrap())
    };
    let date = Date { year: year, month: month, day: day };

    let author = match captures.get(4) {
        Some(user_name) => {
            HistoryEntryAuthor::UserName(user_name.as_str().to_string())
        },
        None => {
            HistoryEntryAuthor::RealName(captures.get(5)
                .unwrap()
                .as_str()
                .to_string())
        },
    };
    let text = captures.get(6).unwrap().as_str().to_string();

    Ok(Meta::History(HistoryEntry { date: date, author: author, text: text }))
}

/// Parse a line containing a !KEYWORDS Meta statement.
fn parse_meta_keywords(path: &Path,
                       line_number: u32,
                       fields: Vec<&str>)
    -> ParseResult1<Meta> {
    check_fields_ge(path, line_number, &fields, 3)?;
    let keywords: Vec<String> = fields[2..fields.len()]
        .join(" ")
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    Ok(Meta::Keywords(keywords))
}

/// Parse a line containing a !LICENSE Meta statement.
fn parse_meta_license(path: &Path,
                      line_number: u32,
                      fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_full_line(path, line_number, fields, Meta::License)
}

/// Parse a line containing a Name Meta statement.
fn parse_meta_name(path: &Path,
                   line_number: u32,
                   fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_full_line(path, line_number, fields, Meta::Name)
}

/// Parse a line containing a NOFILE Meta statement.
fn parse_meta_nofile(path: &Path,
                     line_number: u32,
                     fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_argumentless(path, line_number, fields, Meta::NoFile)
}

/// Parse a line containing a PAUSE Meta statement.
fn parse_meta_pause(path: &Path,
                    line_number: u32,
                    fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_argumentless(path, line_number, fields, Meta::Pause)
}

/// Parse a line containing a PRINT/WRITE Meta statement.
fn parse_meta_print(path: &Path,
                    line_number: u32,
                    fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_full_line(path, line_number, fields, Meta::Print)
}

/// Parse a line containing a SAVE Meta statement.
fn parse_meta_save(path: &Path,
                   line_number: u32,
                   fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_argumentless(path, line_number, fields, Meta::Save)
}

/// Parse a line containing a STEP Meta statement.
fn parse_meta_step(path: &Path,
                   line_number: u32,
                   fields: Vec<&str>)
    -> ParseResult1<Meta> {
    parse_meta_argumentless(path, line_number, fields, Meta::Step)
}

/// Parse a line containing a Meta statement.
fn parse_meta_statement(path: &Path,
                        line_number: u32,
                        fields: Vec<&str>)
    -> ParseResult1<Statement> {
    type MetaParser = fn(&Path, u32, Vec<&str>) -> ParseResult1<Meta>;
    static META_KEYWORDS: phf::Map<&'static str, &'static MetaParser> =
        phf_map! {
            "!CATEGORY" => &(parse_meta_category as MetaParser),
            "!CMDLINE" => &(parse_meta_cmdline as MetaParser),
            "!COLOUR" => &(parse_meta_color as MetaParser),
            "!HELP" => &(parse_meta_help as MetaParser),
            "!HISTORY" => &(parse_meta_history as MetaParser),
            "!KEYWORDS" => &(parse_meta_keywords as MetaParser),
            "!LDRAW_ORG" => &(parse_meta_file_type as MetaParser),
            "!LICENSE" => &(parse_meta_license as MetaParser),
            "//" => &(parse_meta_comment as MetaParser),
            "Author:" => &(parse_meta_author as MetaParser),
            "BFC" => &(parse_meta_bfc as MetaParser),
            "CLEAR" => &(parse_meta_clear as MetaParser),
            "FILE" => &(parse_meta_file as MetaParser),
            "LDRAW_ORG" => &(parse_meta_file_type as MetaParser),
            "Name:" => &(parse_meta_name as MetaParser),
            "NOFILE" => &(parse_meta_nofile as MetaParser),
            "Official" => &(parse_meta_file_type as MetaParser),
            "PAUSE" => &(parse_meta_pause as MetaParser),
            "SAVE" => &(parse_meta_save as MetaParser),
            "STEP" => &(parse_meta_step as MetaParser),
            "Unofficial" => &(parse_meta_file_type as MetaParser),
            "Un-official" => &(parse_meta_file_type as MetaParser),
            "WRITE" => &(parse_meta_print as MetaParser),
        };

    if fields.len() < 2 {
        return Ok(Statement::Meta(Meta::Empty));
    }
    match META_KEYWORDS.get(fields[1]) {
        Some(handler) => {
            handler(path, line_number, fields).map(Statement::Meta)
        },
        None => {
            let meta = if line_number == 1 {
                Meta::Description
            } else {
                Meta::Unknown
            };
            Ok(Statement::Meta(meta(fields[1..fields.len()].join(" "))))
        },
    }
}

/// Parse a line containing a Subfile statement.
#[allow(unused_attributes)]
#[rustfmt_skip]
fn parse_subfile_statement(path: &Path,
                           line_number: u32,
                           fields: Vec<&str>)
    -> ParseResult1<Statement> {
    type RawMatrix = NAMatrix<f64, U4, U4, ArrayStorage<f64, U4, U4>>;
    check_fields_eq(path, line_number, &fields, 15)?;
    let color_ref = parse_color_ref(path, line_number, fields[1])?;
    let origin = map_result_monad(|f| parse_f64(path, line_number, f),
                                  &fields[2..5])?;
    let transform = map_result_monad(|f| parse_f64(path, line_number, f),
                                     &fields[5..14])?;
    let raw_matrix = RawMatrix::new(
        transform[0], transform[1], transform[2], origin[0],
        transform[3], transform[4], transform[5], origin[1],
        transform[6], transform[7], transform[8], origin[2],
        0.0,          0.0,          0.0,          1.0);
    let matrix: Matrix = unsafe { convert_unchecked(raw_matrix) };
    Ok(Statement::Subfile {
        color: color_ref,
        matrix: matrix,
        file: PathBuf::from(fields[14].replace('\\', "/").to_lowercase()),
    })
}

/// Parse a line containing a Line statement.
fn parse_line_statement(path: &Path,
                        line_number: u32,
                        fields: Vec<&str>)
    -> ParseResult1<Statement> {
    check_fields_eq(path, line_number, &fields, 8)?;
    let color_ref = parse_color_ref(path, line_number, fields[1])?;
    let coordinates =
        map_result_monad(|f| parse_f64(path, line_number, f),
                         &fields[2..8])?;
    let a = Point::new(coordinates[0], coordinates[1], coordinates[2]);
    let b = Point::new(coordinates[3], coordinates[4], coordinates[5]);
    Ok(Statement::Line { color: color_ref, line: Line { a: a, b: b } })
}

/// Parse a line containing a Triangle statement.
fn parse_triangle_statement(path: &Path,
                            line_number: u32,
                            fields: Vec<&str>)
    -> ParseResult1<Statement> {
    check_fields_eq(path, line_number, &fields, 11)?;
    let color_ref = parse_color_ref(path, line_number, fields[1])?;
    let coordinates =
        map_result_monad(|f| parse_f64(path, line_number, f),
                         &fields[2..11])?;
    let a = Point::new(coordinates[0], coordinates[1], coordinates[2]);
    let b = Point::new(coordinates[3], coordinates[4], coordinates[5]);
    let c = Point::new(coordinates[6], coordinates[7], coordinates[8]);
    Ok(Statement::Triangle {
        color: color_ref,
        triangle: Triangle { a: a, b: b, c: c },
    })
}

/// Parse a line containing a Quad statement.
fn parse_quad_statement(path: &Path,
                        line_number: u32,
                        fields: Vec<&str>)
    -> ParseResult1<Statement> {
    check_fields_eq(path, line_number, &fields, 14)?;
    let color_ref = parse_color_ref(path, line_number, fields[1])?;
    let coordinates =
        map_result_monad(|f| parse_f64(path, line_number, f),
                         &fields[2..14])?;
    let a = Point::new(coordinates[0], coordinates[1], coordinates[2]);
    let b = Point::new(coordinates[3], coordinates[4], coordinates[5]);
    let c = Point::new(coordinates[6], coordinates[7], coordinates[8]);
    let d = Point::new(coordinates[9], coordinates[10], coordinates[11]);
    Ok(Statement::Quad {
        color: color_ref,
        quad: Quad { a: a, b: b, c: c, d: d },
    })
}

/// Parse a line containing an OptionalLine statement.
fn parse_optional_line_statement(path: &Path,
                                 line_number: u32,
                                 fields: Vec<&str>)
    -> ParseResult1<Statement> {
    check_fields_eq(path, line_number, &fields, 14)?;
    let color_ref = parse_color_ref(path, line_number, fields[1])?;
    let coordinates =
        map_result_monad(|f| parse_f64(path, line_number, f),
                         &fields[2..14])?;
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
