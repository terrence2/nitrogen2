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
use crate::Plane;
use absolute_unit::prelude::*;
use glam::DQuat;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct Circle<Unit>
where
    Unit: LengthUnit,
{
    plane: Plane<Unit>,
    center: Pt3<Unit>,
    radius: Length<Unit>,
}

impl<Unit> Circle<Unit>
where
    Unit: LengthUnit,
{
    pub fn from_plane_center_and_radius(
        plane: Plane<Unit>,
        center: Pt3<Unit>,
        radius: Length<Unit>,
    ) -> Self {
        Self {
            plane,
            center,
            radius,
        }
    }

    pub fn radius(&self) -> Length<Unit> {
        self.radius
    }

    pub fn center(&self) -> &Pt3<Unit> {
        &self.center
    }

    pub fn plane(&self) -> &Plane<Unit> {
        &self.plane
    }

    pub fn point_at_angle<T: AngleUnit>(&self, angle: Angle<T>) -> Pt3<Unit> {
        // Find a vector at 90 degrees to the plane normal.
        let n = *self.plane.normal();
        let p = n.any_orthonormal_vector() * self.radius;
        let q = DQuat::from_axis_angle(n, radians!(angle).f64());
        self.center + (q * p)
    }
}
