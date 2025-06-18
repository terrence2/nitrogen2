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
use crate::{Aabb3, Ray};
use absolute_unit::prelude::*;
use num_traits::Zero;

/// Result of an intersection between a ray and an AABB.
// TODO: track exit point
#[derive(Clone, Copy, Debug)]
pub enum AabbRayIntersect<Unit: LengthUnit> {
    None,
    Inside { position: Pt3<Unit> },
    Through { closest: Pt3<Unit> },
}

impl<Unit: LengthUnit> AabbRayIntersect<Unit> {
    /// Return an option with the result status.
    #[inline]
    pub fn maybe_entrance(&self) -> Option<&Pt3<Unit>> {
        match self {
            Self::None => None,
            Self::Inside { position } => Some(position),
            Self::Through { closest } => Some(closest),
        }
    }

    /// Panic if the intersection is none.
    #[inline]
    pub fn entrance(&self) -> &Pt3<Unit> {
        match self {
            Self::None => panic!("called closest on a None aabb/ray intersection"),
            Self::Inside { position } => position,
            Self::Through { closest } => closest,
        }
    }

    #[inline]
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
enum Quadrant {
    Middle = 0,
    Left = 1,
    Right = 2,
}

// from https://web.archive.org/web/20090803054252/http://tog.acm.org/resources/GraphicsGems/gems/RayBox.c
pub fn intersect_aabb_ray_into<Unit: LengthUnit>(
    aabb: &Aabb3<Unit>,
    ray: &Ray<Unit>,
    result: &mut AabbRayIntersect<Unit>,
) {
    let mut inside = true;
    let mut quadrant = [Quadrant::Middle; 3];
    let mut candidate_plane = [Length::<Unit>::zero(); 3];

    // Find candidate planes
    for i in 0..3 {
        if ray.origin().at(i) < aabb.low(i) {
            quadrant[i] = Quadrant::Left;
            candidate_plane[i] = aabb.low(i);
            inside = false;
        } else if ray.origin().at(i) > aabb.high(i) {
            quadrant[i] = Quadrant::Right;
            candidate_plane[i] = aabb.high(i);
            inside = false;
        }
    }

    // Ray origin inside bounding box
    if inside {
        *result = AabbRayIntersect::Inside {
            position: *ray.origin(),
        };
        return;
    }

    // Calculate t distance to candidate planes
    let mut max_t = [-1.; 3];
    for i in 0..3 {
        if quadrant[i] != Quadrant::Middle && ray.direction()[i] != 0. {
            max_t[i] = (candidate_plane[i] - ray.origin().at(i)).f64() / ray.direction()[i];
        }
    }

    // Get the largest of the max_t's to find the closest intersection
    let mut which_plane = 0;
    for i in 1..3 {
        // Note: short loop
        if max_t[which_plane] < max_t[i] {
            which_plane = i;
        }
    }

    // Box is behind the ray and ray points away from all planes
    if max_t[which_plane] < 0. {
        *result = AabbRayIntersect::None;
        return;
    }

    // Check that our closest plane intersection lies within the box's side
    let mut position = Pt3::<Unit>::zero();
    for (i, candidate_plane) in candidate_plane.iter().enumerate() {
        if which_plane != i {
            let p =
                ray.origin().at(i) + Length::<Unit>::from(max_t[which_plane] * ray.direction()[i]);
            if p < aabb.low(i) || p > aabb.high(i) {
                *result = AabbRayIntersect::None;
                return;
            }
            position.set(i, p);
        } else {
            position.set(i, *candidate_plane);
        }
    }
    *result = AabbRayIntersect::Through { closest: position };
}

#[inline]
pub fn intersect_ray_aabb_into<Unit: LengthUnit>(
    ray: &Ray<Unit>,
    aabb: &Aabb3<Unit>,
    result: &mut AabbRayIntersect<Unit>,
) {
    intersect_aabb_ray_into(aabb, ray, result);
}

#[inline]
pub fn intersect_aabb_ray<Unit: LengthUnit>(
    aabb: &Aabb3<Unit>,
    ray: &Ray<Unit>,
) -> AabbRayIntersect<Unit> {
    let mut result = AabbRayIntersect::None;
    intersect_aabb_ray_into(aabb, ray, &mut result);
    result
}

#[inline]
pub fn intersect_ray_aabb<Unit: LengthUnit>(
    ray: &Ray<Unit>,
    aabb: &Aabb3<Unit>,
) -> AabbRayIntersect<Unit> {
    let mut result = AabbRayIntersect::None;
    intersect_ray_aabb_into(ray, aabb, &mut result);
    result
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;
    use glam::DVec3;

    fn aabbox() -> Aabb3<Meters> {
        Aabb3::from_bounds(
            Pt3::new(meters!(-1), meters!(-1), meters!(-1)),
            Pt3::new(meters!(1), meters!(1), meters!(1)),
        )
    }

    #[test]
    fn check_ray_aabb_inside() {
        let mut intersect = AabbRayIntersect::None;
        intersect_ray_aabb_into(
            &Ray::new(Pt3::<Meters>::zero(), DVec3::X),
            &aabbox(),
            &mut intersect,
        );
        assert!(intersect.is_some());
        assert_relative_eq!(intersect.maybe_entrance().unwrap().x(), meters!(0.));
        assert_relative_eq!(intersect.maybe_entrance().unwrap().y(), meters!(0.));
        assert_relative_eq!(intersect.maybe_entrance().unwrap().z(), meters!(0.));
    }

    #[test]
    fn check_ray_aabb_intersect() {
        let mut intersect = AabbRayIntersect::None;
        intersect_aabb_ray_into(
            &aabbox(),
            &Ray::new(
                Pt3::new(meters!(2), meters!(2), meters!(2)),
                DVec3::new(-1., -1., -1.).normalize(),
            ),
            &mut intersect,
        );
        assert!(intersect.is_some());
        assert_relative_eq!(intersect.entrance().x(), meters!(1.));
        assert_relative_eq!(intersect.entrance().y(), meters!(1.));
        assert_relative_eq!(intersect.entrance().z(), meters!(1.));
    }

    #[test]
    fn check_ray_aabb_behind() {
        let intersect = intersect_ray_aabb(
            &Ray::new(
                Pt3::new(meters!(2), meters!(2), meters!(2)),
                DVec3::new(1., 1., 1.).normalize(),
            ),
            &aabbox(),
        );
        assert!(intersect.is_none());
    }

    #[test]
    fn check_ray_aabb_parallel() {
        let intersect = intersect_aabb_ray(
            &aabbox(),
            &Ray::new(Pt3::new(meters!(2), meters!(0), meters!(0)), DVec3::X),
        );
        assert!(intersect.is_none());
    }
}
