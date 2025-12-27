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

//! Contents of LDraw files.
//!
//! Following [LDraw.org][1] standards:
//!
//! * [File Format 1.0.2][2]
//! * [List of Official META Commands][3]
//! * [Official Library Header Specification][4]
//! * [CATEGORY and KEYWORDS Language Extension][5]
//! * [Language Extension for Back Face Culling (BFC)][6]
//! * [MPD Language Extension[7]
//!
//! [1]: http://www.ldraw.org
//! [2]: http://www.ldraw.org/article/218
//! [3]: http://www.ldraw.org/article/401
//! [4]: http://www.ldraw.org/article/398
//! [5]: http://www.ldraw.org/article/340
//! [6]: http://www.ldraw.org/article/415
//! [7]: http://www.ldraw.org/article/47

use std::path::PathBuf;

use super::types::*;

/// BFC declaration.
#[derive(Debug)]
pub enum BFCDeclaration {
    Certify(RotationSense),
    NoCertify,
    Rotation(RotationSense),
    Clip(Option<RotationSense>),
    NoClip,
    InvertNext,
}

/// Date.
#[derive(Debug)]
pub struct Date {
    pub year: u16,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

/// Release.
#[derive(Debug)]
pub struct Release {
    pub year: u16,
    pub release: u8,
}

/// File update tag.
#[derive(Debug)]
pub enum UpdateTag {
    Original,
    Date(Date),
    Release(Release),
}

/// File officiality.
#[derive(Debug, Eq, PartialEq)]
pub enum Officiality {
    LDrawOfficial,
    Unofficial,
}

/// File contents.
#[derive(Clone, Copy, Debug)]
pub enum Contents {
    Configuration,
    Part,
    Subpart,
    Primitive,
    Primitive48,
    Primitive8,
    Shortcut,
    File,
    Model,
    Submodel,
    Element,
    SubPart,
    HiResPrimitive,
    Alias,
    CrossReference,
}

/// File qualifier.
#[derive(Debug)]
pub enum Qualifier {
    Alias,
    PhysicalColor,
}

/// Full file type.
#[derive(Debug)]
pub struct FileType {
    pub officiality: Officiality,
    pub contents: Option<Contents>,
    pub qualifiers: Vec<Qualifier>,
    pub update_tag: Option<UpdateTag>,
}

/// History entry author.
#[derive(Debug)]
pub enum HistoryEntryAuthor {
    UserName(String),
    RealName(String),
}

/// History entry.
#[derive(Debug)]
pub struct HistoryEntry {
    pub date: Date,
    pub author: HistoryEntryAuthor,
    pub text: String,
}

/// LDraw meta directives.
#[derive(Debug)]
pub enum Meta {
    /// Author.
    Author(String),

    /// BFC declaration.
    BFC(BFCDeclaration),

    /// Category.
    Category(String),

    /// Clear the screen.
    Clear,

    /// LDraw command-line arguments.
    CmdLine(Vec<String>),

    /// Color declaration.
    Color(Color),

    /// Comment.
    Comment(String),

    /// Description.
    Description(String),

    /// Empty declaration,
    Empty,

    /// File.
    File(String),

    /// File type.
    FileType(FileType),

    /// Help string.
    Help(String),

    /// History entry.
    History(HistoryEntry),

    /// Keywords.
    Keywords(Vec<String>),

    /// License.
    License(String),

    /// Name.
    Name(String),

    /// No file.
    NoFile,

    /// Pause the drawing.
    Pause,

    /// Print/write a message.
    Print(String),

    /// Save a bitmap of the current drawing.
    Save,

    /// Mark the end of a building step.
    Step,

    /// Unknown directive.
    Unknown(String),
}

/// LDraw statement.
#[derive(Debug)]
pub enum Statement {
    Meta(Meta),
    Subfile {
        color: ColorRef,
        matrix: Matrix,
        file: PathBuf,
    },
    Line {
        color: ColorRef,
        line: Line,
    },
    Triangle {
        color: ColorRef,
        triangle: Triangle,
    },
    Quad {
        color: ColorRef,
        quad: Quad,
    },
    OptionalLine {
        color: ColorRef,
        line: Line,
        control_line: Line,
    },
}

/// LDraw file.
#[derive(Debug)]
pub struct LDFile {
    pub statements: Vec<Statement>,
}

// Local Variables:
// coding: utf-8
// End:
