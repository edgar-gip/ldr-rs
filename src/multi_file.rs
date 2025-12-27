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

//! Multi-part LDraw file.
//!
//! Following [LDraw.org][1] standards:
//!
//! * [MPD Language Extension[2]
//!
//! [1]: http://www.ldraw.org
//! [2]: http://www.ldraw.org/article/47

use super::file::{LDFile, Meta, Statement};

/// Part inside a multi-part file.
pub struct LDMultiFilePart {
    pub name: Option<String>,
    pub range_start: usize,
    pub range_end: usize,
}

/// Multi-part file.
pub struct LDMultiFile {
    pub file: LDFile,
    pub main_part: LDMultiFilePart,
    pub parts: Vec<LDMultiFilePart>,
}

impl LDMultiFile {
    /// Creates a new multi-part file.
    pub fn new(file: LDFile) -> Self {
        if file.statements.is_empty() {
            let main_part = LDMultiFilePart {
                name: None,
                range_start: 0,
                range_end: file.statements.len(),
            };
            LDMultiFile {
                file: file,
                main_part: main_part,
                parts: Vec::new(),
            }
        } else if let Statement::Meta(Meta::File(name)) = &file.statements[0] {
            let mut part_name = name;
            let mut part_start = 1;
            let mut parts = Vec::new();
            for index in 1..file.statements.len() {
                match &file.statements[index] {
                    Statement::Meta(Meta::File(name)) => {
                        parts.push(LDMultiFilePart {
                            name: Some(part_name.to_ascii_lowercase()),
                            range_start: part_start,
                            range_end: index,
                        });
                        part_name = name;
                        part_start = index + 1;
                    }
                    Statement::Meta(Meta::NoFile) => {
                        parts.push(LDMultiFilePart {
                            name: Some(part_name.to_ascii_lowercase()),
                            range_start: part_start,
                            range_end: index,
                        });
                        break;
                    }
                    _ => {}
                }
            }
            parts.push(LDMultiFilePart {
                name: Some(part_name.to_ascii_lowercase()),
                range_start: part_start,
                range_end: file.statements.len(),
            });
            LDMultiFile {
                file: file,
                main_part: parts.remove(0),
                parts: parts,
            }
        } else {
            let main_part = LDMultiFilePart {
                name: None,
                range_start: 0,
                range_end: file.statements.len(),
            };
            LDMultiFile {
                file: file,
                main_part: main_part,
                parts: Vec::new(),
            }
        }
    }

    /// Returns the part with the given name inside the multi-part file.
    pub fn find_part(&self, name: &str) -> Option<&LDMultiFilePart> {
        self.parts.iter().find(|part| {
            if let Some(part_name) = &part.name {
                part_name == name
            } else {
                false
            }
        })
    }
}

// Local Variables:
// coding: utf-8
// End:
