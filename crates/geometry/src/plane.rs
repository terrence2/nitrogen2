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
use crate::algorithm::compute_normal;
use absolute_unit::prelude::*;
use approx::relative_eq;
use glam::{DVec3, DVec4, Vec2, Vec3, Vec4Swizzles};
use std::fmt::Debug;

// (x,y,z,1) dot (nx, ny, nz, -d) => ax + by + cz = d
// Normal and negative distance gives the full definition.
#[derive(Clone, Copy, Debug)]
pub struct Plane<Unit>
where
    Unit: LengthUnit,
{
    normal: DVec3,
    distance: Length<Unit>,
}

impl<Unit> Plane<Unit>
where
    Unit: LengthUnit,
{
    pub fn xy() -> Self {
        Self {
            normal: DVec3::Z,
            distance: 0_f64.into(),
        }
    }

    pub fn yz() -> Self {
        Self {
            normal: DVec3::X,
            distance: 0_f64.into(),
        }
    }

    pub fn xz() -> Self {
        Self {
            normal: DVec3::Y,
            distance: 0_f64.into(),
        }
    }

    pub fn from_point_and_normal(point: Pt3<Unit>, normal: DVec3) -> Self {
        debug_assert!(normal.is_normalized());
        Self {
            normal,
            distance: point.dvec3().dot(normal).into(),
        }
    }

    pub fn from_normal_and_distance(normal: DVec3, distance: Length<Unit>) -> Self {
        Self { normal, distance }
    }

    // From 3 points on the plane with clockwise winding
    pub fn from_triangle(pt0: &Pt3<Unit>, pt1: &Pt3<Unit>, pt2: &Pt3<Unit>) -> Self {
        let normal = compute_normal(pt0, pt1, pt2);
        Self::from_point_and_normal(*pt0, normal)
    }

    pub fn from_raw(a: f64, b: f64, c: f64, d: f64) -> Self {
        Self {
            normal: DVec3::new(a, b, c).normalize(),
            distance: Length::<Unit>::from(-d),
        }
    }

    // Note: normalizes
    pub fn from_dvec4(v: DVec4) -> Self {
        let n = v.xyz().length();
        let v = v / n;
        Self::from_raw(v.x, v.y, v.z, v.w)
    }

    pub fn point_on_plane(&self, p: &Pt3<Unit>) -> bool {
        relative_eq!(self.normal.dot(p.dvec3()) - self.distance.f64(), 0_f64)
    }

    pub fn distance_to_point(&self, p: &Pt3<Unit>) -> Length<Unit> {
        (self.normal.dot(p.dvec3()) - self.distance.f64()).into()
    }

    pub fn closest_point_on_plane(&self, p: &Pt3<Unit>) -> Pt3<Unit> {
        *p - (self.normal * self.distance_to_point(p)).pt3()
    }

    pub fn point_is_in_front(&self, p: &Pt3<Unit>) -> bool {
        self.normal.dot(p.dvec3()) - self.distance.f64() >= 0_f64
    }

    pub fn point_is_in_front_rel(&self, p: &Pt3<Unit>, epsilon: Length<Unit>) -> bool {
        self.normal.dot(p.dvec3()) - self.distance().f64() >= epsilon.f64()
    }

    // Returns true when v and n are opposed.
    pub fn vector_points_at(&self, v: &DVec3) -> bool {
        self.normal.dot(*v) < 0.
    }

    pub fn normal(&self) -> &DVec3 {
        &self.normal
    }

    pub fn normal_v3(&self) -> V3<Length<Unit>> {
        V3::new_dvec3(self.normal)
    }

    pub fn distance(&self) -> Length<Unit> {
        self.distance
    }

    pub fn d(&self) -> Length<Unit> {
        -self.distance
    }

    pub fn nudged(mut self, delta: Length<Unit>) -> Self {
        self.distance += delta;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_point_on_plane() {
        let plane = Plane::from_point_and_normal(Pt3::zero(), DVec3::Z);
        assert!(plane.point_on_plane(&Pt3::new(meters!(10f64), meters!(10f64), meters!(0f64))));
        assert!(!plane.point_on_plane(&Pt3::new(meters!(10f64), meters!(10f64), meters!(0.1f64))));
        assert!(!plane.point_on_plane(&Pt3::new(meters!(10f64), meters!(10f64), -meters!(0.1f64))));
    }

    #[test]
    fn test_point_distance() {
        let plane = Plane::from_point_and_normal(Pt3::zero(), DVec3::Z);

        assert_relative_eq!(
            meters!(-1f64),
            plane.distance_to_point(&Pt3::new(meters!(1f64), meters!(1f64), meters!(-1f64)))
        );
        assert_relative_eq!(
            meters!(1f64),
            plane.distance_to_point(&Pt3::new(meters!(-1f64), meters!(-1f64), meters!(1f64)))
        );
    }

    #[test]
    fn test_closest_point_on_plane() {
        let plane = Plane::from_point_and_normal(Pt3::zero(), DVec3::Z);

        assert_relative_eq!(
            Pt3::new(meters!(1f64), meters!(1f64), meters!(0f64)),
            plane.closest_point_on_plane(&Pt3::new(meters!(1f64), meters!(1f64), meters!(-1f64)))
        );
        assert_relative_eq!(
            Pt3::new(meters!(-1f64), meters!(-1f64), meters!(0f64)),
            plane.closest_point_on_plane(&Pt3::new(meters!(-1f64), meters!(-1f64), meters!(1f64)))
        );
    }
}
