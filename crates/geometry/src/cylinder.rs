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
use crate::{Face, Primitive, RenderPrimitive, Vertex};
use absolute_unit::{Length, LengthUnit, Pt3, Volume, scalar};
use glam::{DQuat, DVec3};
use std::{f64, f64::consts::PI};

#[derive(Clone, Debug)]
pub struct Cylinder<Unit: LengthUnit> {
    origin: Pt3<Unit>,
    axis: DVec3,
    length: Length<Unit>,
    radius_bottom: Length<Unit>,
    radius_top: Length<Unit>,
}

impl<Unit: LengthUnit> Cylinder<Unit> {
    pub fn new(origin: Pt3<Unit>, axis: DVec3, length: Length<Unit>, radius: Length<Unit>) -> Self {
        Self {
            origin,
            axis,
            length,
            radius_bottom: radius,
            radius_top: radius,
        }
    }

    pub fn new_tapered(
        origin: Pt3<Unit>,
        axis: DVec3,
        length: Length<Unit>,
        radius_bottom: Length<Unit>,
        radius_top: Length<Unit>,
    ) -> Self {
        Self {
            origin,
            axis,
            length,
            radius_bottom,
            radius_top,
        }
    }

    pub fn axis(&self) -> &DVec3 {
        &self.axis
    }

    pub fn origin(&self) -> &Pt3<Unit> {
        &self.origin
    }

    pub fn set_axis(&mut self, axis: DVec3) {
        self.axis = axis;
    }

    pub fn set_origin(&mut self, origin: Pt3<Unit>) {
        self.origin = origin;
    }

    pub fn radius_bottom(&self) -> Length<Unit> {
        self.radius_bottom
    }

    pub fn radius_top(&self) -> Length<Unit> {
        self.radius_top
    }

    // Axial length
    pub fn length(&self) -> Length<Unit> {
        self.length
    }

    pub fn volume(&self) -> Volume<Unit> {
        // Note: despite being called cylinder, this is actually the volume
        // of a truncated cone.
        let r0 = self.radius_top;
        let r1 = self.radius_bottom;
        let depth = self.length;
        scalar!(1_f64 / 3_f64) * scalar!(PI) * (r0 * r0 + r0 * r1 + r1 * r1) * depth
    }
}

impl<Unit: LengthUnit> RenderPrimitive for Cylinder<Unit> {
    fn to_primitive(&self, detail: u32) -> Primitive {
        // Number of faces on the sides
        let steps = detail;
        let origin = self.origin.dvec3();

        // Build all vertices by subdividing up two circles on +y.
        let mut verts = make_unit_circle(steps, 0_f64, self.radius_bottom.f64());
        let mut top = make_unit_circle(steps, self.length.f64(), self.radius_top.f64());
        verts.append(&mut top);

        // Transform the vertices from +y into the axis basis.
        let facing = DQuat::from_rotation_arc(DVec3::Y, self.axis);
        for vert in &mut verts {
            vert.position = origin + facing * vert.position;
            vert.normal = facing * vert.normal;
            if !vert.normal.is_normalized() {
                vert.normal = DVec3::Y;
            }
        }

        // Build faces
        // Sides
        let mut faces = Vec::new();
        for i in 0..steps {
            let a = i;
            let b = (i + 1) % steps;
            let c = a + steps;
            let d = b + steps;
            faces.push(Face::new(b, a, c, &verts));
            faces.push(Face::new(d, b, c, &verts));
        }
        // bottom cap
        for i in 1..steps {
            faces.push(Face::new_with_normal(
                (i + 1) % steps,
                0,
                i,
                facing * -DVec3::Y,
            ));
        }
        // top cap
        for i in 1..steps {
            faces.push(Face::new_with_normal(
                i + steps,
                steps,
                (i + 1) % steps + steps,
                facing * DVec3::Y,
            ));
        }

        Primitive { verts, faces }
    }
}

fn make_unit_circle(steps: u32, offset: f64, radius: f64) -> Vec<Vertex> {
    let mut out = Vec::new();
    let dr = 2. * PI / steps as f64;
    for i in 0..steps {
        let alpha = dr * i as f64;
        out.push(Vertex {
            position: DVec3::new(alpha.cos() * radius, offset, alpha.sin() * radius),
            normal: DVec3::new(alpha.cos(), 0., alpha.sin()),
        });
    }
    out
}
