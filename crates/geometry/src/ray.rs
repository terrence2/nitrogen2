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
use absolute_unit::{Length, LengthUnit, Pt3};
use glam::DVec3;
// use rapier3d_f64::geometry::Ray as RapRay;
use std::fmt::Debug;

/// Ray-vs-Ray "intersection" values. Captures the closest approach and offset to that point.
pub struct RayApproach<Unit>
where
    Unit: LengthUnit,
{
    pub offset: Length<Unit>,
    pub closest: Pt3<Unit>,
}

#[derive(Clone, Debug)]
pub struct Ray<Unit>
where
    Unit: LengthUnit,
{
    origin: Pt3<Unit>,
    direction: DVec3,
}

impl<Unit> Ray<Unit>
where
    Unit: LengthUnit,
{
    pub fn new(origin: Pt3<Unit>, direction: DVec3) -> Self {
        assert!(direction.is_normalized());
        Self { origin, direction }
    }

    pub fn origin(&self) -> &Pt3<Unit> {
        &self.origin
    }

    pub fn direction(&self) -> &DVec3 {
        &self.direction
    }

    // https://palitri.com/vault/stuff/maths/Rays%20closest%20point.pdf
    pub fn closest_point_to_other(&self, other: &Self) -> RayApproach<Unit> {
        let a = self.direction;
        let b = other.direction;
        let c = *self.origin.to(other.origin).dvec3();

        let a_a = a.dot(a);
        let a_b = a.dot(b);
        let a_c = a.dot(c);
        let b_b = b.dot(b);
        let b_c = b.dot(c);

        let t = (a_c * b_b - a_b * b_c) / (a_a * b_b - a_b * a_b);
        let offset = Length::<Unit>::from(t);

        RayApproach {
            offset,
            closest: self.origin + offset * self.direction,
        }
    }
}

// impl From<Ray<Meters>> for RapRay {
//     fn from(ray: Ray<Meters>) -> Self {
//         Self::from(&ray)
//     }
// }
// 
// impl From<&Ray<Meters>> for RapRay {
//     fn from(ray: &Ray<Meters>) -> Self {
//         RapRay::new(ray.origin().dvec3().into(), (*ray.direction()).into())
//     }
// }
