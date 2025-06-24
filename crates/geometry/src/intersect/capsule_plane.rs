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
use crate::{
    Capsule, Plane,
    intersect::{PlaneSegmentIntersect, intersect_plane_segment},
};
use absolute_unit::prelude::*;
use glam::DVec3;

/// Intersection between a capsule and a plane.
#[derive(Clone, Copy, Debug)]
pub enum CapsulePlaneIntersect<Unit: LengthUnit> {
    None,

    // Capsule is fully above the Plane (normal side out). Includes approach information.
    Above {
        closest: Pt3<Unit>,
        distance: Length<Unit>,
        normal: DVec3,
    },

    // Capsule is fully below the Plane. Intersect is the point on the plane that would be in
    // contact, given the rejection information is applied.
    Below {
        // Closest point on plane to the *furthest* in point below the plane.
        intersect: Pt3<Unit>,
        // Depth of the furthest point on capsule away from the plane rear.
        interpenetration: Length<Unit>,
        // Depth of the nearest point away from the plane rear.
        depth: Length<Unit>,
        // Normal to the plane.
        normal: DVec3,
    },

    // The capsule has penetrated the plane; intersection is the closest pivot point
    // on the capsule and rejection information.
    Intersect {
        intersect: Pt3<Unit>,
        interpenetration: Length<Unit>,
        normal: DVec3,
    },

    // The capsule is penetrated into the plane, but parallel.
    CylinderIntersect {
        center: Pt3<Unit>,
        interpenetration: Length<Unit>,
        normal: DVec3,
    },
}

impl<Unit: LengthUnit> CapsulePlaneIntersect<Unit> {
    /// Panic if the intersection is None
    #[inline]
    pub fn intersect(&self) -> &Pt3<Unit> {
        match self {
            Self::None => panic!("called intersect on a none capsule/plane intersect"),
            Self::Above { .. } => panic!("called intersect on an above capsule/plane intersect"),
            Self::Below { intersect, .. } => intersect,
            Self::Intersect { intersect, .. } => intersect,
            Self::CylinderIntersect { center, .. } => center,
        }
    }

    /// Panic if the intersection is None
    #[inline]
    pub fn normal(&self) -> &DVec3 {
        match self {
            Self::None => panic!("called normal on a none capsule/plane intersect"),
            Self::Above { .. } => panic!("called normal on an above capsule/plane intersect"),
            Self::Below { normal, .. } => normal,
            Self::Intersect { normal, .. } => normal,
            Self::CylinderIntersect { normal, .. } => normal,
        }
    }

    /// Panic if the intersection is None
    #[inline]
    pub fn interpenetration(&self) -> Length<Unit> {
        match self {
            Self::None => panic!("called interpenetration on a none capsule/plane intersect"),
            Self::Above { .. } => {
                panic!("called interpenetration on an above capsule/plane intersect")
            }
            Self::Below {
                interpenetration, ..
            } => *interpenetration,
            Self::Intersect {
                interpenetration, ..
            } => *interpenetration,
            Self::CylinderIntersect {
                interpenetration, ..
            } => *interpenetration,
        }
    }

    #[inline]
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        debug_assert!(!matches!(self, Self::None));
        matches!(self, Self::Above { .. })
    }
}

pub fn intersect_capsule_plane_into<Unit: LengthUnit>(
    capsule: &Capsule<Unit>,
    plane: &Plane<Unit>,
    result: &mut CapsulePlaneIntersect<Unit>,
) {
    match intersect_plane_segment(plane, capsule.segment()) {
        PlaneSegmentIntersect::None => {
            panic!("plane/segment intersect none in capsule plane intersect")
        }
        PlaneSegmentIntersect::Above {
            closest,
            distance,
            normal,
        } => {
            if &distance > capsule.radius() {
                // No contact above, cut the radius out of the distance.
                *result = CapsulePlaneIntersect::Above {
                    closest,
                    distance: distance - *capsule.radius(),
                    normal,
                };
            } else {
                // Contact is still the closest point, interpenetration is the amount extra
                *result = CapsulePlaneIntersect::Intersect {
                    intersect: closest,
                    interpenetration: *capsule.radius() - distance,
                    normal,
                };
            }
        }
        PlaneSegmentIntersect::Below {
            intersect,
            interpenetration,
            depth,
            normal,
        } => {
            // This may be below or intersecting, depending on the amount it's below past
            // the plane. We can capture this as the 'depth', like we did for Above.
            if depth > *capsule.radius() {
                // Capsule is fully below the plane
                *result = CapsulePlaneIntersect::Below {
                    intersect,
                    interpenetration: interpenetration + *capsule.radius(),
                    depth: depth - *capsule.radius(),
                    normal,
                };
            } else {
                // Capsule is breaching at the rear
                *result = CapsulePlaneIntersect::Intersect {
                    intersect,
                    interpenetration: interpenetration + *capsule.radius(),
                    normal,
                };
            }
        }
        PlaneSegmentIntersect::Intersect {
            intersect,
            interpenetration,
            normal,
            ..
        } => {
            *result = CapsulePlaneIntersect::Intersect {
                intersect,
                interpenetration: interpenetration + *capsule.radius(),
                normal,
            }
        }
        PlaneSegmentIntersect::Coplanar { segment } => {
            *result = CapsulePlaneIntersect::Intersect {
                intersect: segment.center(),
                interpenetration: *capsule.radius(),
                normal: *plane.normal(),
            }
        }
    }
}

#[inline]
pub fn intersect_plane_capsule_into<Unit: LengthUnit>(
    plane: &Plane<Unit>,
    capsule: &Capsule<Unit>,
    result: &mut CapsulePlaneIntersect<Unit>,
) {
    intersect_capsule_plane_into(capsule, plane, result)
}

#[inline]
pub fn intersect_capsule_plane<Unit: LengthUnit>(
    capsule: &Capsule<Unit>,
    plane: &Plane<Unit>,
) -> CapsulePlaneIntersect<Unit> {
    let mut result = CapsulePlaneIntersect::None;
    intersect_capsule_plane_into(capsule, plane, &mut result);
    result
}

#[inline]
pub fn intersect_plane_capsule<Unit: LengthUnit>(
    plane: &Plane<Unit>,
    capsule: &Capsule<Unit>,
) -> CapsulePlaneIntersect<Unit> {
    let mut result = CapsulePlaneIntersect::None;
    intersect_plane_capsule_into(plane, capsule, &mut result);
    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Segment;
    use approx::assert_relative_eq;

    fn capsule(
        (ax, ay, az): (f64, f64, f64),
        (bx, by, bz): (f64, f64, f64),
        r: f64,
    ) -> Capsule<Meters> {
        Capsule::new(
            Segment::new(
                Pt3::new(meters!(ax), meters!(ay), meters!(az)),
                Pt3::new(meters!(bx), meters!(by), meters!(bz)),
            ),
            meters!(r),
        )
    }

    #[test]
    fn test_capsule_above_plane() {
        let intersect =
            intersect_capsule_plane(&capsule((0., 2., 0.), (0., 4., 0.), 1.), &Plane::xz());
        assert!(intersect.is_none());
    }

    #[test]
    fn test_capsule_below_plane() {
        let intersect =
            intersect_plane_capsule(&Plane::xz(), &capsule((0., -2., 0.), (0., -4., 0.), 1.));
        assert!(intersect.is_some());
        assert_relative_eq!(intersect.intersect(), &Pt3::<Meters>::new_unit(0., 0., 0.));
        assert!(matches!(intersect, CapsulePlaneIntersect::Below { .. }));
        assert_relative_eq!(intersect.interpenetration(), meters!(5));
    }

    #[test]
    fn test_capsule_tangent_above_plane() {
        let mut intersect = CapsulePlaneIntersect::None;
        intersect_plane_capsule_into(
            &Plane::xz(),
            &capsule((0., 1., 0.), (0., 3., 0.), 1.),
            &mut intersect,
        );
        assert!(intersect.is_some());
        assert_relative_eq!(intersect.intersect(), &Pt3::<Meters>::new_unit(0., 0., 0.));
        assert!(matches!(intersect, CapsulePlaneIntersect::Intersect { .. }));
        assert_relative_eq!(intersect.normal(), &DVec3::Y);
        assert_relative_eq!(intersect.interpenetration(), meters!(0));
    }

    #[test]
    fn test_capsule_start_near_plane() {
        let mut intersect = CapsulePlaneIntersect::None;
        intersect_plane_capsule_into(
            &Plane::xz(),
            &capsule((0., 0.5, 0.), (0., 2.5, 0.), 1.),
            &mut intersect,
        );
        assert!(intersect.is_some());
        assert_relative_eq!(intersect.intersect(), &Pt3::<Meters>::new_unit(0., 0., 0.));
        assert!(matches!(intersect, CapsulePlaneIntersect::Intersect { .. }));
        assert_relative_eq!(intersect.normal(), &DVec3::Y);
        assert_relative_eq!(intersect.interpenetration(), meters!(0.5));
    }

    #[test]
    fn test_capsule_end_near_plane() {
        let mut intersect = CapsulePlaneIntersect::None;
        intersect_plane_capsule_into(
            &Plane::xz(),
            &capsule((0., 0.5, 0.), (0., 2.5, 0.), 1.),
            &mut intersect,
        );
        assert!(intersect.is_some());
        assert_relative_eq!(intersect.intersect(), &Pt3::<Meters>::new_unit(0., 0., 0.));
        assert!(matches!(intersect, CapsulePlaneIntersect::Intersect { .. }));
        assert_relative_eq!(intersect.normal(), &DVec3::Y);
        assert_relative_eq!(intersect.interpenetration(), meters!(0.5));
    }

    #[test]
    fn test_capsule_tangent_angled() {
        let mut intersect = CapsulePlaneIntersect::None;
        intersect_plane_capsule_into(
            &Plane::xz(),
            &capsule((1., 1., 0.), (1., 2., 0.), 1.),
            &mut intersect,
        );
        assert!(intersect.is_some());
        assert_relative_eq!(intersect.intersect(), &Pt3::<Meters>::new_unit(1., 0., 0.));
        assert!(matches!(intersect, CapsulePlaneIntersect::Intersect { .. }));
    }
}
