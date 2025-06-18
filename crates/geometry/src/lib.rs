// This file is part of Nitrogen.
//
// Nitrogen is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Nitrogen is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Nitrogen.  If not, see <http://www.gnu.org/licenses/>.

mod aabb3;
mod aabb_n;
pub mod algorithm;
mod arrow;
mod capsule;
mod circle;
mod cylinder;
pub mod intersect;
mod plane;
mod ray;
mod segment;
mod sphere;
mod trimesh;

pub use crate::{
    aabb3::Aabb3, aabb_n::Aabb, arrow::Arrow, capsule::Capsule, circle::Circle, cylinder::Cylinder,
    plane::Plane, ray::Ray, segment::Segment, sphere::Sphere, trimesh::TriMesh,
};

use glam::DVec3;

#[derive(Debug)]
pub struct Vertex {
    pub position: DVec3,
    pub normal: DVec3,
}

impl Vertex {
    pub fn new(position: DVec3) -> Self {
        Self {
            position,
            normal: position.normalize(),
        }
    }

    pub fn new_with_normal(position: DVec3, normal: DVec3) -> Self {
        debug_assert!(normal.is_normalized() || normal.length_squared() == 0.);
        Self { position, normal }
    }
}

#[derive(Debug)]
pub struct Face {
    pub index0: u32,
    pub index1: u32,
    pub index2: u32,
    pub normal: DVec3,
}

impl Face {
    pub fn new(i0: u32, i1: u32, i2: u32, verts: &[Vertex]) -> Self {
        let v0 = verts[i0 as usize].position;
        let v1 = verts[i1 as usize].position;
        let v2 = verts[i2 as usize].position;
        let normal = (v1 - v0).cross(v2 - v0).normalize_or_zero();
        Face {
            index0: i0,
            index1: i1,
            index2: i2,
            normal,
        }
    }

    pub fn new_with_normal(i0: u32, i1: u32, i2: u32, normal: DVec3) -> Self {
        Face {
            index0: i0,
            index1: i1,
            index2: i2,
            normal,
        }
    }

    pub fn i0(&self) -> usize {
        self.index0 as usize
    }

    pub fn i1(&self) -> usize {
        self.index1 as usize
    }

    pub fn i2(&self) -> usize {
        self.index2 as usize
    }
}

#[derive(Debug, Default)]
pub struct Primitive {
    pub verts: Vec<Vertex>,
    pub faces: Vec<Face>,
}

impl Primitive {
    pub fn extend(&mut self, other: &mut Primitive) {
        let offset = self.verts.len() as u32;
        for face in &mut other.faces {
            face.index0 += offset;
            face.index1 += offset;
            face.index2 += offset;
        }
        self.verts.append(&mut other.verts);
        self.faces.append(&mut other.faces);
    }
}

pub trait RenderPrimitive {
    fn to_primitive(&self, detail: u32) -> Primitive;
}
