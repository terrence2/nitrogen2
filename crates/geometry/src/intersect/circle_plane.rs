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
use crate::{Circle, Plane};
use absolute_unit::prelude::*;
use approx::relative_eq;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy)]
pub enum CirclePlaneIntersection<Unit>
where
    Unit: LengthUnit,
{
    Parallel,
    InFrontOfPlane,
    BehindPlane,
    Intersection(Pt3<Unit>, Pt3<Unit>),
    Tangent(Pt3<Unit>),
}

pub fn intersect_circle_plane_into<Unit: LengthUnit>(
    circle: &Circle<Unit>,
    plane: &Plane<Unit>,
    sidedness_offset: Length<Unit>,
    result: &mut CirclePlaneIntersection<Unit>,
) {
    // We can get the direction by crossing normals.
    let d = circle.plane().normal().cross(*plane.normal());

    // Detect and reject the parallel case: e.g. direction is ~0.
    if relative_eq!(d.dot(d), 0_f64) {
        *result = CirclePlaneIntersection::Parallel;
        return;
    }
    let d = d.normalize();

    // Find the line: the line is orthogonal to both normals and has direction d.
    // Taken from the clever code here:
    //   https://stackoverflow.com/questions/6408670/line-of-intersection-between-two-planes
    let p = ((d.cross(*plane.normal()) * circle.plane().d())
        + (circle.plane().normal().cross(d) * plane.d()))
    .pt3();

    // Project circle center onto new line.
    let t = Length::<Unit>::from((*circle.center() - p).dvec3().dot(d));
    let p_closest = p + d * t;
    let closest_distance = (*circle.center() - p_closest).length();
    if closest_distance > circle.radius() {
        *result = if plane.point_is_in_front_rel(circle.center(), sidedness_offset) {
            CirclePlaneIntersection::InFrontOfPlane
        } else {
            CirclePlaneIntersection::BehindPlane
        };
        return;
    }
    if relative_eq!(closest_distance, circle.radius()) {
        *result = CirclePlaneIntersection::Tangent(p_closest);
        return;
    }

    // Apply pythagoras to get the distance from p_closest to our two roots.
    let t1 = (circle.radius() * circle.radius() - closest_distance * closest_distance).sqrt();
    *result = CirclePlaneIntersection::Intersection(p_closest + d * t1, p_closest - (d * t1).pt3());
}

#[inline]
pub fn intersect_plane_circle_into<Unit: LengthUnit>(
    plane: &Plane<Unit>,
    circle: &Circle<Unit>,
    sidedness_offset: Length<Unit>,
    result: &mut CirclePlaneIntersection<Unit>,
) {
    intersect_circle_plane_into(circle, plane, sidedness_offset, result);
}

#[inline]
pub fn intersect_circle_plane<Unit: LengthUnit>(
    circle: &Circle<Unit>,
    plane: &Plane<Unit>,
    sidedness_offset: Length<Unit>,
) -> CirclePlaneIntersection<Unit> {
    let mut result = CirclePlaneIntersection::Parallel;
    intersect_circle_plane_into(circle, plane, sidedness_offset, &mut result);
    result
}

#[inline]
pub fn intersect_plane_circle<Unit: LengthUnit>(
    plane: &Plane<Unit>,
    circle: &Circle<Unit>,
    sidedness_offset: Length<Unit>,
) -> CirclePlaneIntersection<Unit> {
    let mut result = CirclePlaneIntersection::Parallel;
    intersect_plane_circle_into(plane, circle, sidedness_offset, &mut result);
    result
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;
    use glam::DVec3;

    #[test]
    fn it_can_handle_two_points() {
        let c = Circle::from_plane_center_and_radius(
            Plane::from_point_and_normal(
                Pt3::zero(),
                DVec3::Y, // facing up
            ),
            Pt3::zero(), // center at origin
            meters!(1f64),
        );
        let p = Plane::from_point_and_normal(
            Pt3::new_unit(-0.5f64, 0f64, 0f64), // offset 1/2 of radius
            -DVec3::X,                          // facing left
        );

        // From top down:
        //   _
        // /_|  .
        // \ | /|
        //   - -- <- ?? on z axis
        //   /\ -0.5 on x axis

        // -0.5**2 + ??**2 = 1
        // 1 - 0.25 = ??**2
        // sqrt(0.75) = ??
        // = 0.866

        let i = intersect_circle_plane(&c, &p, meters!(0f64));
        println!("i: {i:?}");
        match i {
            CirclePlaneIntersection::Intersection(a, b) => {
                assert_relative_eq!(a.x(), meters!(-0.5_f64));
                assert_relative_eq!(a.y(), meters!(0_f64));
                assert_relative_eq!(a.z(), meters!(0.75_f64.sqrt()));
                assert_relative_eq!(b.x(), meters!(-0.5_f64));
                assert_relative_eq!(b.y(), meters!(0_f64));
                assert_relative_eq!(b.z(), meters!(-(0.75_f64.sqrt())));
            }
            _ => panic!("expected circle-plane intersect"),
        }
    }

    #[test]
    fn it_can_handle_incident_points() {
        let c = Circle::from_plane_center_and_radius(
            Plane::from_point_and_normal(Pt3::zero(), DVec3::Y),
            Pt3::zero(),
            meters!(1f64),
        );
        let p = Plane::from_point_and_normal(Pt3::new_unit(1f64, 0f64, 0f64), -DVec3::X);

        let i = intersect_circle_plane(&c, &p, meters!(0f64));
        match i {
            CirclePlaneIntersection::Tangent(pt) => {
                assert_relative_eq!(pt.x(), meters!(1_f64));
                assert_relative_eq!(pt.y(), meters!(0_f64));
                assert_relative_eq!(pt.z(), meters!(0_f64));
            }
            _ => panic!("expected circle-plane intersect"),
        }
    }

    #[test]
    fn it_can_handle_zero_points() {
        let c = Circle::from_plane_center_and_radius(
            Plane::from_point_and_normal(Pt3::zero(), DVec3::Y),
            Pt3::zero(),
            meters!(1f64),
        );
        let p = Plane::from_point_and_normal(Pt3::new_unit(10f64, 0f64, 0f64), DVec3::X);
        let i = intersect_circle_plane(&c, &p, meters!(0f64));
        assert!(matches!(i, CirclePlaneIntersection::BehindPlane));

        let p = Plane::from_point_and_normal(Pt3::new_unit(10f64, 0f64, 0f64), -DVec3::X);
        let i = intersect_circle_plane(&c, &p, meters!(0f64));
        assert!(matches!(i, CirclePlaneIntersection::InFrontOfPlane));
    }

    #[test]
    fn it_can_handle_parallel_planes() {
        let c = Circle::from_plane_center_and_radius(
            Plane::from_point_and_normal(Pt3::zero(), DVec3::Y),
            Pt3::zero(),
            meters!(1f64),
        );
        let p = Plane::from_point_and_normal(Pt3::new_unit(0f64, 1f64, 0f64), DVec3::Y);
        // Point is a 1 up, why is d -1?

        let i = intersect_circle_plane(&c, &p, meters!(0f64));
        assert!(matches!(i, CirclePlaneIntersection::Parallel));
    }
}
