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
use crate::{Plane, Segment};
use absolute_unit::{Length, LengthUnit, Pt3, scalar};
use approx::relative_eq;
use glam::DVec3;

#[derive(Clone, Debug)]
pub enum PlaneSegmentIntersect<Unit: LengthUnit> {
    None,

    // Segment lies on the plane. Use the segment it self for intersection information.
    Coplanar {
        segment: Segment<Unit>,
    },

    // Segment is fully above the plane. Approach information is given.
    Above {
        closest: Pt3<Unit>,
        distance: Length<Unit>,
        normal: DVec3,
    },

    // Segment is fully below the plane. Rejection information is given.
    Below {
        // Closest point on plane to the *furthest* point below the plane.
        intersect: Pt3<Unit>,
        // Depth of the furthest point away from the plane rear.
        interpenetration: Length<Unit>,
        // Depth of the nearest point away from the plane rear.
        depth: Length<Unit>,
        // Normal to the plane.
        normal: DVec3,
    },

    // Segment crosses the plane. The intersection point is included, along
    // with the contact point if rejected and rejection information.
    Intersect {
        // Point where the segment crosses the plane.
        intersect: Pt3<Unit>,
        // True if the first point in the segment is above the plane.
        start_is_in_front: bool,
        // Depth of penetration of the deepest point.
        interpenetration: Length<Unit>,
        // Closest point on plane to the *furthest* point below the plane.
        contact: Pt3<Unit>,
        // Normal to the plane.
        normal: DVec3,
    },
}

impl<Unit: LengthUnit> PlaneSegmentIntersect<Unit> {
    // Panic if the segment does not intersect
    #[inline]
    pub fn intersect(&self) -> Pt3<Unit> {
        match self {
            Self::None => panic!("uninitialized plane segment intersection"),
            Self::Above { .. } => panic!("called intersect on non-intersecting plane/segment"),
            Self::Coplanar { segment } => segment.center(),
            Self::Below { intersect, .. } => *intersect,
            Self::Intersect { intersect, .. } => *intersect,
        }
    }

    #[inline]
    pub fn interpenetration(&self) -> Length<Unit> {
        match self {
            Self::None => panic!("uninitialized plane segment intersection"),
            Self::Above { .. } => panic!("called intersect on non-intersecting plane/segment"),
            Self::Coplanar { .. } => (0.).into(),
            Self::Below {
                interpenetration, ..
            } => *interpenetration,
            Self::Intersect {
                interpenetration, ..
            } => *interpenetration,
        }
    }

    /// Panic if intersects
    #[inline]
    pub fn closest_approach(&self) -> &Pt3<Unit> {
        match self {
            Self::None => panic!("uninitialized plane segment intersection"),
            Self::Above { closest, .. } => closest,
            _ => panic!("called closest on intersecting segment-plane"),
        }
    }

    /// Panic if intersects
    #[inline]
    pub fn approach_distance(&self) -> &Length<Unit> {
        match self {
            Self::None => panic!("uninitialized plane segment intersection"),
            Self::Above { distance, .. } => distance,
            _ => panic!("called distance on intersecting segment-plane"),
        }
    }

    pub fn is_in_contact(&self) -> bool {
        match self {
            Self::None => panic!("uninitialized plane segment intersection"),
            Self::Above { .. } => false,
            _ => true,
        }
    }

    pub fn is_intersecting(&self) -> bool {
        match self {
            Self::None => panic!("uninitialized plane segment intersection"),
            Self::Intersect { .. } => true,
            _ => false,
        }
    }
}

pub fn intersect_plane_segment_into<Unit: LengthUnit>(
    plane: &Plane<Unit>,
    segment: &Segment<Unit>,
    result: &mut PlaneSegmentIntersect<Unit>,
) {
    let n = *plane.normal();
    let s = n * plane.distance();
    let o = *segment.start();

    let top = (s - o.v3()).dvec3().dot(n);
    let bottom = segment.direction().dot(n);

    // Parallel to the plane
    if relative_eq!(bottom, 0.) {
        let d = plane.distance_to_point(segment.start());
        if relative_eq!(d.f64(), 0., epsilon = 0.000_001) {
            *result = PlaneSegmentIntersect::Coplanar {
                segment: segment.to_owned(),
            };
        } else if d.f64() < 0. {
            *result = PlaneSegmentIntersect::Below {
                intersect: plane.closest_point_on_plane(&segment.center()),
                interpenetration: d,
                depth: d,
                normal: *plane.normal(),
            }
        } else {
            *result = PlaneSegmentIntersect::Above {
                closest: plane.closest_point_on_plane(&segment.center()),
                distance: d,
                normal: *plane.normal(),
            }
        }
        return;
    }

    let t = top / bottom;
    if t < 0. || t > segment.length().f64() {
        let d0 = plane.distance_to_point(segment.start());
        let d1 = plane.distance_to_point(segment.end());

        // Segment fully behind
        if d0.f64() < 0. {
            debug_assert!(d1.f64() < 0.);
            if d0 > d1 {
                // Note: Both are negative, so start is closer to the back of the plane.
                //       Hence, the intersection matches the rejection point, which is the
                //       more interpenetrated point in this case.
                *result = PlaneSegmentIntersect::Below {
                    intersect: plane.closest_point_on_plane(segment.end()),
                    depth: -d0,
                    interpenetration: -d1,
                    normal: *plane.normal(),
                };
            } else {
                // End closer to back of plane, start is interpenetration contact.
                *result = PlaneSegmentIntersect::Below {
                    intersect: plane.closest_point_on_plane(segment.start()),
                    interpenetration: -d0,
                    depth: -d1,
                    normal: *plane.normal(),
                };
            }
            return;
        } else {
            // Segment fully in front
            debug_assert!(d0.f64() > 0.);
            debug_assert!(d1.f64() > 0.);
            if d0 < d1 {
                // Note: both positive, so start is closer to the front of the plane.
                *result = PlaneSegmentIntersect::Above {
                    closest: plane.closest_point_on_plane(segment.start()),
                    distance: d0,
                    normal: *plane.normal(),
                };
            } else {
                // End closer to front of plane
                *result = PlaneSegmentIntersect::Above {
                    closest: plane.closest_point_on_plane(segment.end()),
                    distance: d1,
                    normal: *plane.normal(),
                };
            }
            return;
        }
    }

    // Otherwise we have a direct intersection.
    let intersect = o + scalar!(t) * segment.direction_v3();
    let start_is_in_front = plane.point_is_in_front(segment.start());
    let inside = if start_is_in_front {
        segment.end()
    } else {
        segment.start()
    };
    let interpenetration = -plane.distance_to_point(inside);
    let contact = plane.closest_point_on_plane(inside);
    *result = PlaneSegmentIntersect::Intersect {
        intersect,
        start_is_in_front,
        interpenetration,
        contact,
        normal: *plane.normal(),
    };
}

#[inline]
pub fn intersect_segment_plane_into<Unit: LengthUnit>(
    segment: &Segment<Unit>,
    plane: &Plane<Unit>,
    result: &mut PlaneSegmentIntersect<Unit>,
) {
    intersect_plane_segment_into(plane, segment, result);
}

#[inline]
pub fn intersect_plane_segment<Unit: LengthUnit>(
    plane: &Plane<Unit>,
    segment: &Segment<Unit>,
) -> PlaneSegmentIntersect<Unit> {
    let mut result = PlaneSegmentIntersect::None;
    intersect_plane_segment_into(plane, segment, &mut result);
    result
}

#[inline]
pub fn intersect_segment_plane<Unit: LengthUnit>(
    segment: &Segment<Unit>,
    plane: &Plane<Unit>,
) -> PlaneSegmentIntersect<Unit> {
    let mut result = PlaneSegmentIntersect::None;
    intersect_segment_plane_into(segment, plane, &mut result);
    result
}

#[cfg(test)]
mod test {
    use super::*;
    use absolute_unit::{Meters, meters};

    fn segment((ax, ay, az): (f64, f64, f64), (bx, by, bz): (f64, f64, f64)) -> Segment<Meters> {
        Segment::new(
            Pt3::new(meters!(ax), meters!(ay), meters!(az)),
            Pt3::new(meters!(bx), meters!(by), meters!(bz)),
        )
    }

    #[test]
    fn test_segment_above_plane_start() {
        let result = intersect_segment_plane(&segment((0., 1., 0.), (0., 2., 1.)), &Plane::xz());
        assert!(!result.is_in_contact());
        assert_eq!(*result.approach_distance(), meters!(1.));
        assert_eq!(
            *result.closest_approach(),
            Pt3::<Meters>::new_unit(0., 0., 0.)
        );
    }

    #[test]
    fn test_segment_above_plane_end() {
        let result = intersect_segment_plane(&segment((0., 2., 1.), (0., 1., 0.)), &Plane::xz());
        assert!(!result.is_in_contact());
        assert_eq!(*result.approach_distance(), meters!(1.));
        assert_eq!(
            *result.closest_approach(),
            Pt3::<Meters>::new_unit(0., 0., 0.)
        );
    }

    #[test]
    fn test_segment_coplanar() {
        let result = intersect_segment_plane(&segment((0., 0., 1.), (0., 0., -1.)), &Plane::xz());
        assert!(result.is_in_contact());
        assert_eq!(result.interpenetration(), meters!(0.));
        assert_eq!(
            result.intersect(),
            Pt3::<Meters>::new_unit(0., 0., 0.) // center
        );
    }

    #[test]
    fn test_segment_intersect_start() {
        let result = intersect_segment_plane(&segment((0., -1., 1.), (0., 1., -1.)), &Plane::xz());
        assert!(result.is_in_contact());
        assert_eq!(result.interpenetration(), meters!(1.));
        assert_eq!(
            result.intersect(),
            Pt3::<Meters>::new_unit(0., 0., 0.) // center
        );
    }
}
