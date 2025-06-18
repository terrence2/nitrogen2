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
use crate::Ray;
use absolute_unit::prelude::*;
use approx::abs_diff_eq;
use num_traits::Float;

#[derive(Debug)]
pub struct RayTriangleIntersect<T: LengthUnit> {
    distance: Length<T>,
    position: Pt3<T>,
    u: f64,
    v: f64,
}

impl<T: LengthUnit> RayTriangleIntersect<T> {
    pub fn distance<U: LengthUnit>(&self) -> Length<U> {
        Length::<U>::from(&self.distance)
    }

    pub fn position(&self) -> &Pt3<T> {
        &self.position
    }

    pub fn u(&self) -> f64 {
        self.u
    }

    pub fn v(&self) -> f64 {
        self.v
    }

    pub fn w(&self) -> f64 {
        1. - self.u - self.v
    }
}

/// Directly from: https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm
pub fn ray_vs_triangle<T: LengthUnit>(
    ray: &Ray<T>,
    p0: &Pt3<T>,
    p1: &Pt3<T>,
    p2: &Pt3<T>,
) -> Option<RayTriangleIntersect<T>> {
    let e1 = *p1 - *p0;
    let e2 = *p2 - *p0;

    let dir_cross_e2 = ray.direction().cross(e2.dvec3());
    let det = e1.dvec3().dot(dir_cross_e2);

    // Ray parallel to plane
    if abs_diff_eq!(det, 0., epsilon = f64::epsilon()) {
        return None;
    }

    // Solve for u
    let inv_det = 1. / det;
    let s = *ray.origin() - *p0;
    let u = inv_det * s.dvec3().dot(dir_cross_e2);

    #[allow(clippy::manual_range_contains)]
    if u < 0. || u > 1. {
        return None;
    }

    // Solve for v
    let s_cross_e1 = s.dvec3().cross(e1.dvec3());
    let v = inv_det * ray.direction().dot(s_cross_e1);

    if v < 0. || u + v > 1. {
        return None;
    }

    // Compute intersection position
    let t = inv_det * e2.dvec3().dot(s_cross_e1);
    if t > f64::epsilon() {
        let intersection: Pt3<T> = *ray.origin() + V3::<Length<T>>::new_dvec3(*ray.direction() * t);
        return Some(RayTriangleIntersect {
            position: intersection,
            distance: Length::<T>::from(t),
            u,
            v,
        });
    }

    // Behind the ray
    None
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;
    use glam::DVec3;

    #[test]
    fn check_ray_triangle_intersect() {
        let intersect = ray_vs_triangle(
            &Ray::new(Pt3::<Meters>::zero(), DVec3::X),
            &Pt3::<Meters>::new_unit(10., 5., 5.),
            &Pt3::<Meters>::new_unit(10., 5., -5.),
            &Pt3::<Meters>::new_unit(10., -5., 0.),
        );
        assert!(intersect.is_some());
        assert_relative_eq!(intersect.as_ref().unwrap().position().x(), meters!(10.));
        assert_relative_eq!(intersect.as_ref().unwrap().position().y(), meters!(0.));
        assert_relative_eq!(intersect.as_ref().unwrap().position().z(), meters!(0.));
    }

    #[test]
    fn check_ray_triangle_behind() {
        let intersect = ray_vs_triangle(
            &Ray::new(Pt3::<Meters>::zero(), -DVec3::X),
            &Pt3::<Meters>::new_unit(10., 5., 5.),
            &Pt3::<Meters>::new_unit(10., 5., -5.),
            &Pt3::<Meters>::new_unit(10., -5., 0.),
        );
        assert!(intersect.is_none());
    }

    #[test]
    fn check_ray_triangle_parallel() {
        let intersect = ray_vs_triangle(
            &Ray::new(Pt3::<Meters>::zero(), DVec3::Y),
            &Pt3::<Meters>::new_unit(10., 5., 5.),
            &Pt3::<Meters>::new_unit(10., 5., -5.),
            &Pt3::<Meters>::new_unit(10., -5., 0.),
        );
        assert!(intersect.is_none());
    }
}
