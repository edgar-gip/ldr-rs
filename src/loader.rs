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

//! Loader of LDraw models.

use na;
use na::core::dimension::U4;
use na::core::{ArrayStorage, Matrix as NAMatrix};
use std::collections::{BTreeMap, HashMap};
use std::convert::From;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::file::{Meta, Statement};
use super::load::{self, ParseError, ParseResult};
use super::multi_file::LDMultiFile;
use super::types::{Color, ColorRef, Line, Matrix, Quad, Triangle, MAIN_COLOR_INDEX};

/// Options to initialize a loader.
pub struct Options {
    /// Path to the LDraw file tree root.
    pub ldraw_path: PathBuf,
}

/// Loader of LDraw models.
pub struct Loader {
    /// Options.
    options: Options,

    /// Mapping from relative to full paths.
    full_paths: HashMap<PathBuf, PathBuf>,

    /// Mapping from full paths to files.
    files: HashMap<PathBuf, Rc<LDMultiFile>>,
}

/// State of the loader while loading a model.
struct State<'a, 'b> {
    /// Current path.
    path: &'a Path,

    /// Current part within the file.
    part_name: Option<String>,

    /// Directory where the root file was located.
    root_directory: &'b Path,

    /// Current mapping from color codes to colors.
    colors: BTreeMap<u16, Color>,

    /// Current transformation matrix.
    transform: Matrix,
}

impl<'a, 'b> State<'a, 'b> {
    /// Creates a new root State.
    fn new(path: &'a Path, root_directory: &'b Path) -> State<'a, 'b> {
        type RawMatrix = NAMatrix<f64, U4, U4, ArrayStorage<f64, U4, U4>>;
        let colors = BTreeMap::new();
        let raw_transform = RawMatrix::new(
            1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        );
        let transform: Matrix = unsafe { na::convert_unchecked(raw_transform) };
        State {
            path: path,
            part_name: None,
            root_directory: root_directory,
            colors: colors,
            transform: transform,
        }
    }

    /// Creates a new substate for a new path.
    fn substate_path<'c>(&self, path: &'c Path, color: &Color, matrix: &Matrix) -> State<'c, 'b> {
        let mut new_colors = self.colors.clone();
        new_colors.insert(MAIN_COLOR_INDEX, color.clone());
        State {
            path: path,
            part_name: None,
            root_directory: self.root_directory,
            colors: new_colors,
            transform: self.transform * matrix,
        }
    }

    /// Creates a new substate for a new part.
    fn substate_part(&self, part_name: String, color: &Color, matrix: &Matrix) -> State<'a, 'b> {
        let mut new_colors = self.colors.clone();
        new_colors.insert(MAIN_COLOR_INDEX, color.clone());
        State {
            path: self.path,
            part_name: Some(part_name),
            root_directory: self.root_directory,
            colors: new_colors,
            transform: self.transform * matrix,
        }
    }

    /// Resolves a color reference to a color, according to the current mapping.
    fn resolve_color_ref(&self, color_ref: &ColorRef) -> ParseResult<&Color> {
        match color_ref {
            ColorRef::Indexed(index) => {
                if let Some(color) = self.colors.get(&index) {
                    Ok(color)
                } else {
                    let error = ParseError {
                        path: self.path.to_path_buf(),
                        line_number: 0,
                        error: format!("Can't find color index {}", index),
                    };
                    Err(vec![error])
                }
            }
            ColorRef::RGB(_) => {
                let error = ParseError {
                    path: self.path.to_path_buf(),
                    line_number: 0,
                    error: String::from("RBG color references not supported"),
                };
                Err(vec![error])
            }
        }
    }

    /// Transforms a line, according to the current transformation matrix.
    fn transform_line(&self, line: &Line) -> Line {
        Line {
            a: self.transform * line.a,
            b: self.transform * line.b,
        }
    }

    /// Transforms a triangle, according to the current transformation matrix.
    fn transform_triangle(&self, triangle: &Triangle) -> Triangle {
        Triangle {
            a: self.transform * triangle.a,
            b: self.transform * triangle.b,
            c: self.transform * triangle.c,
        }
    }

    /// Transforms a quad, according to the current transformation matrix.
    fn transform_quad(&self, quad: &Quad) -> Quad {
        Quad {
            a: self.transform * quad.a,
            b: self.transform * quad.b,
            c: self.transform * quad.c,
            d: self.transform * quad.d,
        }
    }
}

/// Interface for visitors of LDraw models.
pub trait Visitor {
    /// Visits a line.
    fn visit_line(&mut self, color: &Color, line: &Line);

    /// Visits a triangle.
    fn visit_triangle(&mut self, color: &Color, triangle: &Triangle);

    /// Visits a quad.
    fn visit_quad(&mut self, color: &Color, quad: &Quad);

    /// Visits an optional line.
    fn visit_optional_line(&mut self, color: &Color, line: &Line, control_line: &Line);
}

impl Loader {
    /// Creates a new loader.
    pub fn new(options: Options) -> Loader {
        let full_paths = HashMap::new();
        let files = HashMap::new();
        Loader {
            options: options,
            full_paths: full_paths,
            files: files,
        }
    }

    /// Accepts the visitor on the model rooted at the given path.
    pub fn accept<V: Visitor>(&mut self, path: &Path, visitor: &mut V) -> ParseResult<()> {
        let root_directory = if path.is_relative() {
            PathBuf::from(".")
        } else if let Some(parent) = path.parent() {
            parent.to_path_buf()
        } else {
            let error = ParseError {
                path: path.to_path_buf(),
                line_number: 0,
                error: format!("{} is not a file", path.display()),
            };
            return Err(vec![error]);
        };
        let ldconfig_path = Path::new("LDConfig.ldr");
        let mut state = State::new(&ldconfig_path, &root_directory);
        self.accept_rec(&mut state, visitor)?;
        state.path = path;
        self.accept_rec(&mut state, visitor)?;
        Ok(())
    }

    /// Expands a potentially relative path to a full one.
    fn expand_path<'a>(
        full_paths: &'a mut HashMap<PathBuf, PathBuf>,
        options: &Options,
        state: &State,
    ) -> ParseResult<&'a PathBuf> {
        let path = state.path;
        if !full_paths.contains_key(path) {
            if path.is_absolute() {
                full_paths.insert(path.to_path_buf(), path.to_path_buf());
            } else {
                let full_path = state.root_directory.join(path);
                if let Ok(_) = fs::metadata(&full_path) {
                    full_paths.insert(path.to_path_buf(), full_path);
                } else {
                    for subdir in vec![".", "p", "p/48", "parts", "parts/s"] {
                        let full_path = options.ldraw_path.join(subdir).join(path);
                        if let Ok(_) = fs::metadata(&full_path) {
                            full_paths.insert(path.to_path_buf(), full_path);
                            break;
                        }
                    }
                }
            }
        }
        let error = ParseError {
            path: path.to_path_buf(),
            line_number: 0,
            error: format!("Can't find {}", path.display()),
        };
        full_paths.get(path).ok_or(vec![error])
    }

    /// Loads a file, specified as a potentially relative path, resolving that
    /// path if needed.
    fn load_file<'a, 'b>(
        files: &'a mut HashMap<PathBuf, Rc<LDMultiFile>>,
        full_paths: &'b mut HashMap<PathBuf, PathBuf>,
        options: &Options,
        state: &State,
    ) -> ParseResult<Rc<LDMultiFile>> {
        let full_path = Loader::expand_path(full_paths, options, state)?;
        if !files.contains_key(full_path) {
            let new_file = load::load_ldraw(full_path)?;
            files.insert(
                state.path.to_path_buf(),
                Rc::new(LDMultiFile::new(new_file)),
            );
        }
        Ok(files.get(state.path).unwrap().clone())
    }

    /// Accepts the visitor on the model recursively.
    fn accept_rec<V: Visitor>(&mut self, state: &mut State, visitor: &mut V) -> ParseResult<()> {
        let multi_file =
            Loader::load_file(&mut self.files, &mut self.full_paths, &self.options, state)?;
        let part = match &state.part_name {
            None => &multi_file.main_part,
            Some(name) => multi_file.find_part(name).ok_or_else(|| {
                vec![ParseError {
                    path: state.path.to_path_buf(),
                    line_number: 0,
                    error: format!("Can't find {} in {}", name, state.path.display()),
                }]
            })?,
        };
        for statement in multi_file.file.statements[part.range_start..part.range_end].iter() {
            match statement {
                Statement::Meta(meta) => {
                    self.process_meta(state, &meta);
                }
                Statement::Subfile {
                    color: color_ref,
                    matrix,
                    file,
                } => {
                    let color = state.resolve_color_ref(color_ref)?;
                    let lower_file = file.to_str().unwrap().to_ascii_lowercase();
                    let mut new_state = if multi_file.find_part(&lower_file).is_some() {
                        state.substate_part(lower_file, &color, &matrix)
                    } else {
                        state.substate_path(&file, &color, &matrix)
                    };
                    self.accept_rec(&mut new_state, visitor)?;
                }
                Statement::Line {
                    color: color_ref,
                    line,
                } => {
                    let color = state.resolve_color_ref(color_ref)?;
                    let transformed_line = state.transform_line(line);
                    visitor.visit_line(color, &transformed_line);
                }
                Statement::Triangle {
                    color: color_ref,
                    triangle,
                } => {
                    let color = state.resolve_color_ref(color_ref)?;
                    let transformed_triangle = state.transform_triangle(triangle);
                    visitor.visit_triangle(color, &transformed_triangle);
                }
                Statement::Quad {
                    color: color_ref,
                    quad,
                } => {
                    let color = state.resolve_color_ref(color_ref)?;
                    let transformed_quad = state.transform_quad(quad);
                    visitor.visit_quad(color, &transformed_quad);
                }
                Statement::OptionalLine {
                    color: color_ref,
                    line,
                    control_line,
                } => {
                    let color = state.resolve_color_ref(color_ref)?;
                    let transformed_line = state.transform_line(line);
                    let transformed_control_line = state.transform_line(control_line);
                    visitor.visit_optional_line(
                        color,
                        &transformed_line,
                        &transformed_control_line,
                    );
                }
            }
        }
        Ok(())
    }

    /// Updates the current state based on a meta directive.
    fn process_meta(&self, state: &mut State, meta: &Meta) {
        match meta {
            Meta::Color(color) => {
                state.colors.insert(color.code, color.clone());
            }
            _ => {}
        }
    }
}

// Local Variables:
// coding: utf-8
// End:
