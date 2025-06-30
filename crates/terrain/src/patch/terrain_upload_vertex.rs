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
use absolute_unit::{Meters, Pt3, Radians};
use geodesy::Geodetic;
use glam::DVec3;
use std::mem;
use zerocopy::{FromBytes, Immutable, IntoBytes};

#[repr(C)]
#[derive(IntoBytes, FromBytes, Immutable, Copy, Clone, Debug, Default)]
pub struct TerrainUploadVertex {
    position: [f32; 3],
    normal: [f32; 3],
    graticule: [f32; 2],
}

impl TerrainUploadVertex {
    pub fn empty() -> Self {
        Self {
            position: [0f32; 3],
            normal: [0f32; 3],
            graticule: [0f32; 2],
        }
    }

    pub fn new(v_view: &Pt3<Meters>, n0: &DVec3, geodetic: &Geodetic) -> Self {
        Self {
            position: v_view.dvec3().as_vec3().to_array(),
            normal: n0.as_vec3().to_array(),
            graticule: geodetic.to_lat_lon_array::<Radians>(),
        }
    }

    pub fn mem_size() -> usize {
        mem::size_of::<Self>()
    }
}
