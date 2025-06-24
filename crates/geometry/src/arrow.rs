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
use crate::{Cylinder, Primitive, RenderPrimitive};
use absolute_unit::{Length, LengthUnit, Pt3, scalar};
use glam::DVec3;

#[derive(Clone, Debug)]
pub struct Arrow<Unit>
where
    Unit: LengthUnit,
{
    origin: Pt3<Unit>,
    axis: DVec3,
    length: Length<Unit>,
    radius: Length<Unit>,
}

impl<Unit> Arrow<Unit>
where
    Unit: LengthUnit,
{
    pub fn new(origin: Pt3<Unit>, axis: DVec3, length: Length<Unit>, radius: Length<Unit>) -> Self {
        Self {
            origin,
            axis,
            length,
            radius,
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

    pub fn set_length(&mut self, length: Length<Unit>) {
        self.length = length;
    }
}

impl<Unit> RenderPrimitive for Arrow<Unit>
where
    Unit: LengthUnit,
{
    fn to_primitive(&self, detail: u32) -> Primitive {
        let shaft = Cylinder::new(
            self.origin,
            self.axis,
            self.length * scalar!(0.9),
            self.radius,
        );
        let head = Cylinder::new_tapered(
            self.origin + self.axis * (self.length * scalar!(0.9)),
            self.axis,
            self.length * scalar!(0.1),
            self.radius * scalar!(1.4_f64),
            Length::<Unit>::from(0_f64),
        );

        let mut prim = shaft.to_primitive(detail);
        prim.extend(&mut head.to_primitive(detail));
        prim
    }
}
