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

//! Display an LDraw model using Kiss3d.

extern crate kiss3d;
extern crate ldraw;
extern crate nalgebra as na;

use kiss3d::light::Light;
use kiss3d::resource::Mesh;
use kiss3d::window::Window;
use ldraw::{Color, Line, Loader, Options as LoaderOptions, Quad, Triangle,
            Visitor};
use na::{Point3, Vector3};
use std::cell::RefCell;
use std::env;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// LDraw visitor which renders a file on a Kiss3d window.
struct KissVisitor<'a> {
    /// Target Kiss3d window.
    window: &'a mut Window,
}

impl<'a> KissVisitor<'a> {
    /// Creates a new visitor.
    fn new(window: &'a mut Window) -> KissVisitor<'a> {
        KissVisitor { window: window }
    }

    /// Converts a Point3<f64> to a Point3<f32>.
    fn to_f32(p: &Point3<f64>) -> Point3<f32> {
        Point3::new(p.x as f32, p.y as f32, p.z as f32)
    }
}

impl<'a> Visitor for KissVisitor<'a> {
    #[allow(unused_variables)]
    fn visit_line(&mut self, color: &Color, line: &Line) {}

    fn visit_triangle(&mut self, color: &Color, triangle: &Triangle) {
        let a = KissVisitor::to_f32(&triangle.a);
        let b = KissVisitor::to_f32(&triangle.b);
        let c = KissVisitor::to_f32(&triangle.c);
        let coords = vec![a, b, c];
        let faces = vec![Point3::new(0, 1, 2)];
        let normals = None;
        let uvs = None;
        let scale = Vector3::new(1.0, 1.0, 1.0);
        const DYNAMIC_DRAW: bool = false;
        let mesh = Rc::new(RefCell::new(Mesh::new(coords,
                                                  faces,
                                                  normals,
                                                  uvs,
                                                  DYNAMIC_DRAW)));
        let mut obj = self.window.add_mesh(mesh, scale);
        obj.set_color(color.value.0 as f32 / 255.0,
                      color.value.1 as f32 / 255.0,
                      color.value.2 as f32 / 255.0);
        obj.enable_backface_culling(false);
    }

    fn visit_quad(&mut self, color: &Color, quad: &Quad) {
        let a = KissVisitor::to_f32(&quad.a);
        let b = KissVisitor::to_f32(&quad.b);
        let c = KissVisitor::to_f32(&quad.c);
        let d = KissVisitor::to_f32(&quad.d);
        let coords = vec![a, b, c, d];
        let faces = vec![Point3::new(0, 1, 2), Point3::new(0, 2, 3)];
        let normals = None;
        let uvs = None;
        let scale = Vector3::new(1.0, 1.0, 1.0);
        const DYNAMIC_DRAW: bool = false;
        let mesh = Rc::new(RefCell::new(Mesh::new(coords,
                                                  faces,
                                                  normals,
                                                  uvs,
                                                  DYNAMIC_DRAW)));
        let mut obj = self.window.add_mesh(mesh, scale);
        obj.set_color(color.value.0 as f32 / 255.0,
                      color.value.1 as f32 / 255.0,
                      color.value.2 as f32 / 255.0);
        obj.enable_backface_culling(false);
    }

    #[allow(unused_variables)]
    fn visit_optional_line(&mut self,
                           color: &Color,
                           line: &Line,
                           control_line: &Line) {
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() == 3);
    let ref ldraw_path: String = args[1];
    let ref file_path: String = args[2];

    let mut window = Window::new(&file_path);
    {
        let mut visitor = KissVisitor::new(&mut window);
        let loader_options =
            LoaderOptions { ldraw_path: PathBuf::from(ldraw_path) };
        let mut loader = Loader::new(loader_options);
        loader.accept(&Path::new(&file_path), &mut visitor).unwrap();
    }

    window.set_light(Light::StickToCamera);

    while window.render() {}
}

// Local Variables:
// coding: utf-8
// End:
