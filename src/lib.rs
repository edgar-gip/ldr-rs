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

//! [LDraw][1] models for Rust.
//!
//! [1]: http://www.ldraw.org

#![feature(custom_attribute)]
#![feature(proc_macro_hygiene)]

#[macro_use]
extern crate lazy_static;

extern crate nalgebra as na;
extern crate phf;
extern crate regex;

pub mod file;
pub mod load;
pub mod loader;
pub mod types;

pub use self::file::*;
pub use self::load::*;
pub use self::loader::*;
pub use self::types::*;

// Local Variables:
// coding: utf-8
// End:
