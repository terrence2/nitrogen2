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
use absolute_unit::prelude::*;
use bevy::prelude::*;
use geodesy::{earth_radius, everest_height};
use geometry::{
    Plane, Sphere,
    algorithm::{compute_normal, solid_angle},
    intersect,
    intersect::{CirclePlaneIntersection, PlaneSide, SpherePlaneIntersection},
};
use glam::DVec3;
// use marker::Markers;
// use planck::{Color};

// We introduce a substantial amount of error in our intersection computations below
// with all the dot products and re-normalizations. This is fine, as long as we use a
// large enough offset when comparing near zero to get stable results and that pad
// extends the collisions in the right direction.
const SIDEDNESS_OFFSET: f64 = 0f64;

#[derive(Debug, Copy, Clone)]
pub(crate) struct Patch {
    // Planes defining the sides of the "cone". Planes point inward.
    planes: [Plane<Meters>; 3],

    // In geocentric, cartesian meters
    pts: [Pt3<Meters>; 3],
    top_pts: [Pt3<Meters>; 3],
}

impl Patch {
    pub(crate) fn new(pts: [Pt3<Meters>; 3]) -> Self {
        // It's not sufficient to make the box sides everest height because the earth is
        // curved. We know the chord length and radius, so we can compute the amount to extend as:
        // c*c = a*a + b*b
        // r*r = chord/2 * chord/2 + h*h
        // h = sqrt(r*r - (chord/2*chord/2))
        // offset = (r - h) + everest_height()
        let chord = pts[0].to(pts[1]).magnitude();
        let c = chord / scalar!(2);
        let h = (earth_radius() * earth_radius() - c * c).sqrt();
        let offset = earth_radius() - h + everest_height();
        let top_pts = pts.map(|pt| pt + (pt.dvec3().normalize() * offset));

        let origin = Pt3::zero();
        let patch = Self {
            planes: [
                Plane::from_point_and_normal(pts[0], compute_normal(&pts[1], &origin, &pts[0])),
                Plane::from_point_and_normal(pts[1], compute_normal(&pts[2], &origin, &pts[1])),
                Plane::from_point_and_normal(pts[2], compute_normal(&pts[0], &origin, &pts[2])),
            ],
            pts,
            top_pts,
        };
        assert!(patch.planes[0].point_is_in_front(&pts[2]));
        assert!(patch.planes[1].point_is_in_front(&pts[0]));
        assert!(patch.planes[2].point_is_in_front(&pts[1]));
        patch
    }

    pub(crate) fn points(&self) -> &[Pt3<Meters>; 3] {
        &self.pts
    }

    pub(crate) fn edge(&self, i: u8) -> (Pt3<Meters>, Pt3<Meters>) {
        match i {
            0 => (self.pts[0], self.pts[1]),
            1 => (self.pts[1], self.pts[2]),
            2 => (self.pts[2], self.pts[0]),
            _ => unreachable!(),
        }
    }

    pub(crate) fn compute_solid_angle(
        &self,
        viewable_area: &[Plane<Meters>; 6],
        eye_position: &Pt3<Meters>,
        eye_direction: &DVec3,
        gizmos: &mut Option<&mut Gizmos>,
    ) -> f64 {
        if !self.keep(viewable_area) {
            return -1f64;
        }

        let tmp = self.compute_visible_solid_angle(eye_position, eye_direction, gizmos);
        assert!(!tmp.is_nan());
        tmp
    }

    fn color_ramp(sa: f64) -> Color {
        const TOP: f32 = 0.004;
        const BOTTOM: f32 = 0.;
        let f = (sa as f32).clamp(BOTTOM, TOP) / TOP;
        Color::linear_rgb(1. - f, f, 0.)
    }

    // Note: The patch is either in front of or intersecting our frustum, so we always want to
    // count up the intersected angle as positive. This means we need to change the winding for
    // solid_angle based on which side of the face we're on.
    //
    // The area we care about maximizing is the sides + top dome + bottom dome. This would be lots
    // of polys, so we approximate via the flat version.
    fn compute_visible_solid_angle(
        &self,
        eye_position: &Pt3<Meters>,
        eye_direction: &DVec3,
        gizmos: &mut Option<&mut Gizmos>,
    ) -> f64 {
        // Compute a base visibility  for the polygon as a whole.
        let mut sa = solid_angle(
            eye_position,
            eye_direction,
            &[self.pts[0], self.pts[1], self.pts[2]],
        );

        for (i, plane) in self.planes.iter().enumerate() {
            let p0 = &self.pts[i];
            let p1 = &self.pts[(i + 1) % 3];
            let p2 = &self.top_pts[(i + 1) % 3];
            let p3 = &self.top_pts[i];
            if plane.vector_points_at(eye_direction) {
                let sa_sides = if plane.point_is_in_front(eye_position) {
                    solid_angle(eye_position, eye_direction, &[*p3, *p2, *p1, *p0])
                } else {
                    solid_angle(eye_position, eye_direction, &[*p0, *p1, *p2, *p3])
                };
                sa += sa_sides;
            }
        }
        if let Some(gizmos) = gizmos.as_deref_mut() {
            gizmos.linestrip(
                self.pts
                    .iter()
                    .map(|p| (*p + *eye_position).dvec3().as_vec3()),
                Self::color_ramp(sa),
            );
            // markers.draw_polygon(
            //     Self::color_ramp(sa),
            //     &self.pts,
            //     &eye_position.dvec3().normalize(),
            // );
        }
        sa
    }

    // Phrase as the inverse so that all we need to do is figure out if any part of the patch is
    // in front of the view.
    fn is_behind_vis_plane(&self, plane: &Plane<Meters>) -> bool {
        // Patch Extent:
        //   outer: the three planes cutting from geocenter through each pair of points in vertices.
        //   bottom: radius of the planet
        //   top: radius of planet from height of everest

        // Two phases:
        //   1) Convex hull over points
        //   2) Plane-sphere for convex top area

        let epsilon = meters!(SIDEDNESS_OFFSET);

        // bottom points
        for p in &self.pts {
            if plane.point_is_in_front_rel(p, epsilon) {
                return false;
            }
        }
        // top points
        for p in &self.pts {
            let top_point = *p + (p.dvec3().normalize() * meters!(everest_height()));
            if plane.point_is_in_front_rel(&top_point, epsilon) {
                return false;
            }
        }

        // plane vs top sphere
        let top_sphere = Sphere::from_center_and_radius(
            Pt3::zero(),
            meters!(earth_radius()) + meters!(everest_height()),
        );
        let intersection = intersect::sphere_vs_plane(&top_sphere, plane);
        if !self.view_intersection_is_in_cone(intersection) {
            return false;
        }

        // None of the hull is in front of the viewable plane.
        true
    }

    fn view_intersection_is_in_cone(&self, intersection: SpherePlaneIntersection<Meters>) -> bool {
        let epsilon = meters!(SIDEDNESS_OFFSET);

        // The viewable region plane intersected the top sphere, so there is a slice of
        // the planet in view. Now we need to figure out if it's _our_ slice. We need to
        // find out if any of the intersection circle is in our cone: e.g. if there is
        // intersection area in front of _all_ planes. So if it is _behind_ any of our
        // planes, we can return false from the full intersection test.
        match intersection {
            SpherePlaneIntersection::NoIntersection { side, .. } => {
                // If the full sphere is in front of the view plane, we are in the cone
                side == PlaneSide::Above
            }
            SpherePlaneIntersection::Intersection(ref circle) => {
                for plane in &self.planes {
                    let intersect = intersect::intersect_circle_plane(circle, plane, epsilon);
                    match intersect {
                        CirclePlaneIntersection::Parallel => {
                            if !plane.point_is_in_front_rel(circle.center(), epsilon) {
                                return false;
                            }
                        }
                        CirclePlaneIntersection::BehindPlane => {
                            return false;
                        }
                        CirclePlaneIntersection::Tangent(ref p) => {
                            if !self.point_is_in_cone(p) {
                                return false;
                            }
                        }
                        CirclePlaneIntersection::Intersection(ref p0, ref p1) => {
                            if !self.point_is_in_cone(p0) && !self.point_is_in_cone(p1) {
                                return false;
                            }
                        }
                        CirclePlaneIntersection::InFrontOfPlane => {
                            if !self.point_is_in_cone(circle.center()) {
                                return false;
                            }
                        }
                    }
                }

                // No test was behind any of the planes, so we are intersecting
                true
            }
        }
    }

    fn point_is_in_cone(&self, point: &Pt3<Meters>) -> bool {
        let epsilon = meters!(SIDEDNESS_OFFSET);

        for plane in &self.planes {
            if !plane.point_is_in_front_rel(point, epsilon) {
                return false;
            }
        }
        true
    }

    fn keep(&self, viewable_area: &[Plane<Meters>; 6]) -> bool {
        for plane in viewable_area {
            if self.is_behind_vis_plane(plane) {
                return false;
            }
        }

        true
    }
}
