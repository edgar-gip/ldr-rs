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

//! Dump the contents of an LDraw file.

extern crate ldraw;

use ldraw::load_ldraw;
use std::env;
use std::io::{Write, stderr};
use std::path::Path;

fn main() {
    let args = env::args();
    assert!(args.len() == 2);
    let file_path: String = args.last().unwrap();

    match load_ldraw(&Path::new(&file_path)) {
        Err(parse_errors) => {
            for error in parse_errors {
                writeln!(stderr(),
                         "{}:{}: {}",
                         error.path.display(),
                         error.line_number,
                         error.error)
                    .unwrap();
            }
        },
        Ok(file) => println!("{:?}\n", file),
    }
}

// Local Variables:
// coding: utf-8
// End:
