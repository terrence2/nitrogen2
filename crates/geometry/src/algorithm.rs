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
use crate::Vertex;
use absolute_unit::{LengthUnit, Pt3};
use glam::DVec3;
use std::f64::consts::TAU;

// Note: this expects left-handed (e.g. clockwise winding).
// Note: using a constant size results in 10% faster execution of our core loop
pub fn solid_angle<Unit: LengthUnit, const N: usize>(
    observer_position: &Pt3<Unit>,
    observer_direction: &DVec3,
    vertices: &[Pt3<Unit>; N],
) -> f64 {
    // compute projected solid area using Stoke's theorem from Improving Radiosity Solutions
    // through the Use of Analytically Determined Form Factors by Baum, Rushmeier, and Winget
    // (Eq. 9 on pg. 6 (or "330"))

    // integrate over edges
    let mut projarea = 0_f64;
    for (i, v) in vertices.iter().enumerate() {
        let j = (i + 1) % vertices.len();
        let v0 = observer_position.to(*v);
        let v1 = observer_position.to(vertices[j]);
        let tau = v0.cross(v1);
        let v0 = v0.normalize_or_zero();
        let v1 = v1.normalize_or_zero();
        let dotp = v0.dot(v1).clamp(-1_f64, 1_f64);

        let gamma = dotp.acos();
        assert!(gamma.is_finite(), "triangle gamma is infinite");

        let mut tau = tau.normalize();
        tau *= gamma;
        projarea -= observer_direction.dot(tau);
    }

    projarea / TAU
}

pub fn compute_normal<Unit: LengthUnit>(p0: &Pt3<Unit>, p1: &Pt3<Unit>, p2: &Pt3<Unit>) -> DVec3 {
    (*p1 - *p0).cross(*p2 - *p0).normalize_or_zero()
}

pub(crate) fn bisect_edge_verts(v0: &Vertex, v1: &Vertex) -> DVec3 {
    bisect_edge(v0.position, v1.position)
}

pub fn bisect_edge(v0: DVec3, v1: DVec3) -> DVec3 {
    v0 + ((v1 - v0) / 2f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use absolute_unit::Meters;
    use glam::DVec3;

    #[test]
    fn solid_angle_produces_proper_signs_per_winding_and_normal() {
        let p = Pt3::<Meters>::new_unit(0f64, 0f64, 0f64);
        let pts = [
            Pt3::new_unit(0f64, 1f64, 1f64),
            Pt3::new_unit(1f64, 0f64, 1f64),
            Pt3::new_unit(0f64, 0f64, 1f64),
        ];
        let sa = solid_angle(&p, &DVec3::Z, &pts);
        assert!(sa > 0f64);

        // Flipping the vector means we're pointing away from the face, so the SA should be negative.
        let sa = solid_angle(&p, &-DVec3::Z, &pts);
        assert!(sa < 0f64);

        // Similarly, flipping the winding will also flip the sign, back to positive, event
        // though we're still facing away.
        let pts = [
            Pt3::new_unit(0f64, 0f64, 1f64),
            Pt3::new_unit(1f64, 0f64, 1f64),
            Pt3::new_unit(0f64, 1f64, 1f64),
        ];
        let sa = solid_angle(&p, &-DVec3::Z, &pts);
        assert!(sa > 0f64);

        // But if we're actually pointing towards the face again, it is negative, as the face
        // is pointing away.
        let sa = solid_angle(&p, &DVec3::Z, &pts);
        assert!(sa < 0f64);
    }

    #[test]
    fn solid_angle_reduces_with_distance() {
        let p = Pt3::<Meters>::new_unit(0f64, 0f64, 0f64);
        let pts = [
            Pt3::new_unit(0f64, 1f64, 1f64),
            Pt3::new_unit(1f64, 0f64, 1f64),
            Pt3::new_unit(0f64, 0f64, 1f64),
        ];
        let sa0 = solid_angle(&p, &DVec3::Z, &pts);

        let p = Pt3::new_unit(0f64, 0f64, 0.5f64);
        let sa1 = solid_angle(&p, &DVec3::Z, &pts);

        assert!(sa0 < sa1);
    }

    #[test]
    fn solid_angle_scales_with_triangle_area_given_fixed_observer() {
        let p = Pt3::<Meters>::new_unit(0f64, 0f64, 0f64);
        let pts = [
            Pt3::new_unit(0f64, 1f64, 1f64),
            Pt3::new_unit(1f64, 0f64, 1f64),
            Pt3::new_unit(0f64, 0f64, 1f64),
        ];
        let sa0 = solid_angle(&p, &DVec3::Z, &pts);

        let pts = [
            Pt3::new_unit(0f64, 2f64, 1f64),
            Pt3::new_unit(2f64, 0f64, 1f64),
            Pt3::new_unit(0f64, 0f64, 1f64),
        ];
        let sa1 = solid_angle(&p, &DVec3::Z, &pts);

        assert!(sa0 < sa1);
    }
}
