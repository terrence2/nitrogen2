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
// use camera::ScreenCamera;
use geodesy::Geodetic;
use geometry::Plane;
use glam::{DMat4, DVec3};

// The bits from Camera that we need for optimization (e.g. not the target surface).
#[derive(Copy, Clone, Debug)]
pub(crate) struct OptimizeCamera {
    position: Pt3<Meters>,
    forward: DVec3,
    world_to_eye: DMat4,
    frustum: [Plane<Meters>; 5],
}

impl OptimizeCamera {
    pub(crate) fn position<T: LengthUnit>(&self) -> Pt3<T> {
        Pt3::<T>::from(&self.position)
    }

    pub(crate) fn forward(&self) -> &DVec3 {
        &self.forward
    }

    pub(crate) fn world_to_eye_m(&self) -> &DMat4 {
        &self.world_to_eye
    }

    pub(crate) fn world_space_frustum_m(&self) -> &[Plane<Meters>; 5] {
        &self.frustum
    }
}

// FIXME: we need to figure out how to get from a bevy camera into our optimize camera
// impl From<&ScreenCamera> for OptimizeCamera {
//     fn from(camera: &ScreenCamera) -> Self {
//         // Terrain expects to render roughly at the surface. If we start at the
//         // origin, we'll very quickly NaN out.
//         let mut position = camera.position::<Meters>();
//         if camera.position().length() < meters!(10.) {
//             position = Geodetic::new(degrees!(0), degrees!(0), meters!(10)).pt3();
//         }
//         Self {
//             position,
//             forward: *camera.forward(),
//             world_to_eye: camera.world_to_eye::<Meters>(),
//             frustum: camera.world_space_frustum::<Meters>(),
//         }
//     }
// }
