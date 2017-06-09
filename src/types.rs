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

//! LDraw base types.
//!
//! Following [LDraw.org][1] standards:
//!
//! * [File Format 1.0.2][2]
//! * [Colour Definition Language Extension][3].
//!
//! [1]: http://www.ldraw.org
//! [2]: http://www.ldraw.org/article/218
//! [3]: http://www.ldraw.org/article/299

use na::Point3;
use na::geometry::Affine3;

/// Point.
pub type Point = Point3<f64>;

/// Line.
#[derive(Debug)]
pub struct Line {
    pub a: Point,
    pub b: Point,
}

/// Triangle.
#[derive(Debug)]
pub struct Triangle {
    pub a: Point,
    pub b: Point,
    pub c: Point,
}

/// Quadrilateral.
#[derive(Debug)]
pub struct Quad {
    pub a: Point,
    pub b: Point,
    pub c: Point,
    pub d: Point,
}

/// Homogeneous transformation (affine) matrix.
pub type Matrix = Affine3<f64>;

/// Rotation sense.
#[derive(Debug)]
pub enum RotationSense {
    /// Clockwise.
    CW,

    /// Counter-clockwise.
    CCW,
}

/// RGB Color.
#[derive(Debug)]
pub struct RGB(u8, u8, u8);

/// Color reference.
#[derive(Debug)]
pub enum ColorRef {
    Indexed(u16),
    RGB(RGB),
}

/// Glitter material.
#[derive(Debug)]
pub struct Glitter {
    pub value: RGB,
    pub alpha: Option<u8>,
    pub luminance: Option<u8>,
    pub fraction: f64,
    pub v_fraction: f64,
    pub size: (u8, u8), // the two may be equal.
}

/// Speckle material.
#[derive(Debug)]
pub struct Speckle {
    pub value: RGB,
    pub alpha: Option<u8>,
    pub luminance: Option<u8>,
    pub fraction: f64,
    pub size: (u8, u8), // the two may be equal.
}

/// Color material.
#[derive(Debug)]
pub enum Material {
    Glitter(Glitter),
    Speckle(Speckle),
    Unknown(Vec<String>),
}

/// Color finish.
#[derive(Debug)]
pub enum Finish {
    Chrome,
    Pearlescent,
    Rubber,
    MatteMetallic,
    Metal,
    Material(Material),
}

/// Color.
#[derive(Debug)]
pub struct Color {
    pub name: String,
    pub code: u16, // must be 0-511 for LDraw compatibility
    pub value: RGB,
    pub edge: ColorRef,
    pub alpha: Option<u8>,
    pub luminance: Option<u8>,
    pub finish: Option<Finish>,
}

// Local Variables:
// coding: utf-8
// End:
