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

// Module naming convention is the two intersecting primitives sorted alphabetically.
// Each module should provide both ordered functions so that the caller can decide
// which is the primary object.
mod aabb_ray;
mod capsule_plane;
mod circle_plane;
mod plane_segment;
mod plane_sphere;
mod ray_sphere;
mod ray_triangle;

pub use aabb_ray::{
    AabbRayIntersect, intersect_aabb_ray, intersect_aabb_ray_into, intersect_ray_aabb,
    intersect_ray_aabb_into,
};
pub use capsule_plane::{
    CapsulePlaneIntersect, intersect_capsule_plane, intersect_capsule_plane_into,
    intersect_plane_capsule, intersect_plane_capsule_into,
};
pub use circle_plane::{
    CirclePlaneIntersection, intersect_circle_plane, intersect_circle_plane_into,
    intersect_plane_circle, intersect_plane_circle_into,
};
pub use plane_segment::{
    PlaneSegmentIntersect,
    intersect_plane_segment,
    // segment_approach_plane, segment_vs_plane, SegmentPlaneApproachKind, SegmentPlaneIntersect,
    intersect_plane_segment_into,
    intersect_segment_plane,
    intersect_segment_plane_into,
};
pub use plane_sphere::{PlaneSide, SpherePlaneIntersection, sphere_vs_plane};
pub use ray_sphere::sphere_vs_ray;
pub use ray_triangle::ray_vs_triangle;
