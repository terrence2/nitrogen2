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
use crate::{algorithm::compute_normal, Face, Primitive, RenderPrimitive, Vertex};
use absolute_unit::{LengthUnit, Meters, Pt3};
use glam::DQuat;
// use rapier3d_f64::parry::{
//     na::Point3,
//     shape::{Ball, Cuboid, HeightField},
// };

// A trimesh is a small fragment of "loose" triangles. It is intended for
// collision rather than drawing or editing. It contains a pile of points
// and index lists for triangles and edges.
#[derive(Debug)]
pub struct TriMesh<Unit: LengthUnit> {
    points: Vec<Pt3<Unit>>,
    tris: Vec<[usize; 3]>,
    edges: Vec<[usize; 2]>,
}

impl<Unit: LengthUnit> TriMesh<Unit> {
    pub fn new(points: Vec<Pt3<Unit>>, tris: Vec<[usize; 3]>, edges: Vec<[usize; 2]>) -> Self {
        Self {
            points,
            tris,
            edges,
        }
    }

    pub fn tris(&self) -> Vec<[&Pt3<Unit>; 3]> {
        self.tris
            .iter()
            .map(|indices| {
                [
                    &self.points[indices[0]],
                    &self.points[indices[1]],
                    &self.points[indices[2]],
                ]
            })
            .collect()
    }

    pub fn edges(&self) -> Vec<[&Pt3<Unit>; 2]> {
        self.edges
            .iter()
            .map(|indices| [&self.points[indices[0]], &self.points[indices[1]]])
            .collect()
    }

    pub fn transform_by(&mut self, frame: (&Pt3<Meters>, &DQuat)) {
        for pt in &mut self.points {
            *pt = (&(*frame.0 + *frame.1 * *pt)).into();
        }
    }

    // pub fn from_ball(ball: &Ball, detail: u32) -> Self {
    //     Self::from_parry(ball.to_trimesh(detail, detail))
    // }
    // 
    // pub fn from_cuboid(cuboid: &Cuboid) -> Self {
    //     Self::from_parry(cuboid.to_trimesh())
    // }
    // 
    // pub fn from_heightfield(height_field: &HeightField) -> Self {
    //     Self::from_parry(height_field.to_trimesh())
    // }
    // 
    // pub fn from_parry((verts, tris): (Vec<Point3<f64>>, Vec<[u32; 3]>)) -> Self {
    //     let points: Vec<Pt3<Unit>> = verts
    //         .iter()
    //         .map(|v| Pt3::<Unit>::from(&Pt3::<Meters>::new_unit(v.x, v.y, v.z)))
    //         .collect();
    //     let tris: Vec<[usize; 3]> = tris
    //         .iter()
    //         .map(|[a, b, c]| [*c as usize, *b as usize, *a as usize])
    //         .collect();
    //     let mut edges = Vec::new();
    //     for [a, b, c] in tris.iter() {
    //         edges.push([*a, *b]);
    //         edges.push([*b, *c]);
    //         edges.push([*c, *a]);
    //     }
    //     Self {
    //         points,
    //         tris,
    //         edges,
    //     }
    // }
}

impl<Unit> RenderPrimitive for TriMesh<Unit>
where
    Unit: LengthUnit,
{
    // Detail here is level of splitting
    fn to_primitive(&self, _detail: u32) -> Primitive {
        let mut verts = Vec::with_capacity(self.tris.len() * 3);
        let mut faces = Vec::with_capacity(self.tris.len() * 3);
        for [i, j, k] in &self.tris {
            let a = &self.points[*i];
            let b = &self.points[*j];
            let c = &self.points[*k];
            let norm = compute_normal(a, c, b);
            verts.push(Vertex::new_with_normal(a.dvec3(), norm));
            verts.push(Vertex::new_with_normal(c.dvec3(), norm));
            verts.push(Vertex::new_with_normal(b.dvec3(), norm));
            faces.push(Face::new(
                verts.len() as u32 - 3,
                verts.len() as u32 - 2,
                verts.len() as u32 - 1,
                &verts,
            ));
        }
        Primitive { verts, faces }
    }
}
