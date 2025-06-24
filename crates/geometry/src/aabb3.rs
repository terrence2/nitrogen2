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
use crate::{Face, Primitive, RenderPrimitive, Sphere, Vertex};
use absolute_unit::{Length, LengthUnit, Pt3, Volume, scalar};
use glam::DVec3;
// use rapier3d_f64::parry::{na::Isometry3, shape::Cuboid};
use std::{cmp::PartialOrd, fmt::Debug};

#[derive(Clone, Copy, Debug)]
pub struct Aabb3<Unit>
where
    Unit: LengthUnit + PartialOrd,
{
    hi: Pt3<Unit>,
    lo: Pt3<Unit>,
}

impl<Unit> Default for Aabb3<Unit>
where
    Unit: LengthUnit + PartialOrd,
{
    fn default() -> Self {
        Self::empty()
    }
}

impl<Unit> Aabb3<Unit>
where
    Unit: LengthUnit + PartialOrd,
{
    pub fn from_bounds(lo: Pt3<Unit>, hi: Pt3<Unit>) -> Self {
        debug_assert!(lo.x() <= hi.x());
        debug_assert!(lo.y() <= hi.y());
        debug_assert!(lo.z() <= hi.z());
        Self { hi, lo }
    }

    pub fn from_sphere(sphere: &Sphere<Unit>) -> Self {
        let corner = Pt3::new(sphere.radius(), sphere.radius(), sphere.radius());
        Self {
            lo: *sphere.center() - corner,
            hi: *sphere.center() + corner,
        }
    }

    #[inline]
    pub fn empty() -> Self {
        Self {
            hi: Pt3::neg_infinity(),
            lo: Pt3::infinity(),
        }
    }

    #[inline]
    pub fn infinite() -> Self {
        Self {
            hi: Pt3::infinity(),
            lo: Pt3::neg_infinity(),
        }
    }

    #[inline]
    pub fn is_finite(&self) -> bool {
        self.hi.is_finite() && self.lo.is_finite()
    }

    #[inline]
    pub fn is_degenerate(&self) -> bool {
        !self.is_finite()
            || self.hi.x() <= self.lo.x()
            || self.hi.y() <= self.lo.y()
            || self.hi.z() <= self.lo.z()
    }

    #[inline]
    pub fn extend(&mut self, p: &Pt3<Unit>) {
        for i in 0..3 {
            if p.at(i) > self.hi.at(i) {
                self.hi.set(i, p.at(i));
            }
            if p.at(i) < self.lo.at(i) {
                self.lo.set(i, p.at(i));
            }
        }
    }

    #[inline]
    pub fn union(&mut self, other: &Self) {
        for corner in other.corners() {
            self.extend(&corner);
        }
    }

    pub fn contains(&self, p: &Pt3<Unit>) -> bool {
        // All coordinates must be within the bounds of [hi,lo], per axis.
        // If any value on any axis in the point is above or below the box, reject early.
        for i in 0..3 {
            if p.at(i) > self.hi.at(i) || p.at(i) < self.lo.at(i) {
                return false;
            }
        }
        true
    }

    pub fn hi(&self) -> &Pt3<Unit> {
        &self.hi
    }

    pub fn lo(&self) -> &Pt3<Unit> {
        &self.lo
    }

    pub fn hi_as<T: LengthUnit>(&self) -> Pt3<T> {
        Pt3::new(
            self.hi.x_as::<T>(),
            self.hi.y_as::<T>(),
            self.hi.z_as::<T>(),
        )
    }

    pub fn lo_as<T: LengthUnit>(&self) -> Pt3<T> {
        Pt3::new(
            self.lo.x_as::<T>(),
            self.lo.y_as::<T>(),
            self.lo.z_as::<T>(),
        )
    }

    pub fn high(&self, index: usize) -> Length<Unit> {
        self.hi.at(index)
    }

    pub fn low(&self, index: usize) -> Length<Unit> {
        self.lo.at(index)
    }

    pub fn span(&self, index: usize) -> Length<Unit> {
        self.hi.at(index) - self.lo.at(index)
    }

    pub fn as_rescaled(self, scale: f64) -> Self {
        Self {
            lo: (self.lo.v3() * scalar!(scale)).pt3(),
            hi: (self.hi.v3() * scalar!(scale)).pt3(),
        }
    }

    pub fn bounding_sphere<T: LengthUnit>(&self) -> Sphere<T> {
        let diag = self.hi - self.lo;
        let center = self.lo + diag / scalar!(2);
        let radius = diag.length() / scalar!(2);
        Sphere::from_center_and_radius(Pt3::<T>::from(&center), Length::<T>::from(&radius))
    }

    // pub fn cuboid(&self) -> (Isometry3<f64>, Cuboid) {
    //     // Note: cuboid is different than AABB in that AABB center may be off center.
    //     //       As such, we need to include an offset in the local Frame.
    //     let diag = self.hi - self.lo;
    //     let half = diag / scalar!(2);
    //     let center = self.lo + half;
    //     (
    //         Isometry3::new(center.dvec3().into(), Default::default()),
    //         Cuboid::new(half.dvec3().into()),
    //     )
    // }

    pub fn radius<T: LengthUnit>(&self) -> Length<T> {
        let radius = self.hi.length().max(self.lo.length());
        Length::<T>::from(&radius)
    }

    pub fn volume(&self) -> Volume<Unit> {
        self.span(0) * self.span(1) * self.span(2)
    }

    pub fn center(&self) -> Pt3<Unit> {
        self.lo + (self.hi - self.lo) / scalar!(2)
    }

    pub fn corners(&self) -> [Pt3<Unit>; 8] {
        [
            self.lo,
            self.lo.with_x(self.hi.x()),
            self.lo.with_y(self.hi.y()),
            self.lo.with_z(self.hi.z()),
            self.hi,
            self.hi.with_x(self.lo.x()),
            self.hi.with_y(self.lo.y()),
            self.hi.with_z(self.lo.z()),
        ]
    }

    pub fn move_by(&mut self, by: Pt3<Unit>) {
        self.hi += by;
        self.lo += by;
    }
}

impl<Unit> RenderPrimitive for Aabb3<Unit>
where
    Unit: LengthUnit + PartialOrd,
{
    fn to_primitive(&self, _detail: u32) -> Primitive {
        let lo = self.lo.dvec3();
        let hi = self.hi.dvec3();
        let verts = vec![
            Vertex::new(DVec3::new(lo.x, hi.y, lo.z)),
            Vertex::new(DVec3::new(hi.x, hi.y, lo.z)),
            Vertex::new(DVec3::new(hi.x, lo.y, lo.z)),
            Vertex::new(DVec3::new(hi.x, lo.y, hi.z)),
            Vertex::new(DVec3::new(lo.x, hi.y, hi.z)),
            Vertex::new(DVec3::new(lo.x, lo.y, hi.z)),
            Vertex::new(DVec3::new(lo.x, lo.y, lo.z)),
            Vertex::new(DVec3::new(hi.x, hi.y, hi.z)),
        ];
        let [a, b, c, d, e, f, lo, hi] = [0u32, 1, 2, 3, 4, 5, 6, 7];
        let quads = [
            ([lo, a, b, c], -DVec3::Z),
            ([c, b, hi, d], DVec3::X),
            ([d, hi, e, f], DVec3::Z),
            ([f, e, a, lo], -DVec3::X),
            ([a, e, hi, b], DVec3::Y),
            ([f, lo, c, d], -DVec3::Y),
        ];
        let mut faces = Vec::new();
        for ([a, b, c, d], norm) in quads {
            faces.push(Face::new_with_normal(a, b, c, norm));
            faces.push(Face::new_with_normal(a, c, d, norm));
        }

        Primitive { verts, faces }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use absolute_unit::{Meters, meters};

    #[test]
    fn test_degenerate() {
        let a = Aabb3::<Meters>::empty();
        assert!(a.is_degenerate());
        let b = Aabb3::<Meters>::infinite();
        assert!(b.is_degenerate());
        let mut c = Aabb3::empty();
        c.extend(&Pt3::new(meters!(0), meters!(0), meters!(0)));
        assert!(c.is_degenerate());
        let mut d = Aabb3::empty();
        d.extend(&Pt3::new(meters!(0), meters!(0), meters!(0)));
        d.extend(&Pt3::new(meters!(1), meters!(1), meters!(1)));
        assert!(!d.is_degenerate());
    }
}
