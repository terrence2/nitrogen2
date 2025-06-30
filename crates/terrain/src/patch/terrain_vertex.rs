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
use memoffset::offset_of;
use std::mem;
use zerocopy::{FromBytes, Immutable, IntoBytes};

#[repr(C)]
#[derive(IntoBytes, FromBytes, Immutable, Copy, Clone, Default)]
pub struct TerrainVertex {
    surface_position: [f32; 3], // undisplaced position, in view space
    position: [f32; 3],         // displaced position, in view space
    normal: [f32; 3],           // normal, in view space, so that we can displace in view space
    graticule: [f32; 2],        // lat/lon to pass on to the deferred texturing pipeline
}

impl TerrainVertex {
    pub fn empty(_dummy: i32) -> Self {
        Self {
            surface_position: [0f32; 3],
            position: [0f32; 3],
            normal: [0f32; 3],
            graticule: [0f32; 2],
        }
    }

    pub fn mem_size(_dummy: i32) -> usize {
        mem::size_of::<Self>()
    }

    #[allow(clippy::unneeded_field_pattern)]
    pub fn descriptor() -> wgpu::VertexBufferLayout<'static> {
        let tmp = wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // surface_position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                // position
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 1,
                },
                // normal
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 24,
                    shader_location: 2,
                },
                // graticule
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 36,
                    shader_location: 3,
                },
            ],
        };

        assert_eq!(
            tmp.attributes[0].offset,
            offset_of!(TerrainVertex, surface_position) as wgpu::BufferAddress
        );

        assert_eq!(
            tmp.attributes[1].offset,
            offset_of!(TerrainVertex, position) as wgpu::BufferAddress
        );

        assert_eq!(
            tmp.attributes[2].offset,
            offset_of!(TerrainVertex, normal) as wgpu::BufferAddress
        );

        assert_eq!(
            tmp.attributes[3].offset,
            offset_of!(TerrainVertex, graticule) as wgpu::BufferAddress
        );

        assert_eq!(mem::size_of::<TerrainVertex>(), 44);

        tmp
    }
}
