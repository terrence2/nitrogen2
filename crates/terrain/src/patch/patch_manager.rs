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
    Terrain,
    common::OptimizeCamera,
    patch::{
        PatchHandle,
        PatchTree,
        PatchWinding,
        TerrainUploadVertex,
        // TerrainVertex
    },
    tables::{
        get_index_dependency_lut, get_tri_strip_index_range, get_tri_strip_indices,
        get_wireframe_index_buffer,
    },
};
use absolute_unit::prelude::*;
use bevy::{
    prelude::*,
    render::{
        Render,
        // RenderStartup,
        RenderApp,
        RenderSet,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::PipelineCache,
        renderer::{RenderContext, RenderDevice, RenderQueue},
    },
};
use geodesy::Geodetic;
// use mantle::Gpu;
// use marker::Markers;
// use planck::EARTH_RADIUS;
// use shader_shared::{ComputePipelineBuilder, DrawIndexedIndirect, binding, layout::comp};
use static_assertions::{assert_eq_align, assert_eq_size};
use std::{f64::consts::FRAC_PI_2, fmt, mem, ops::Range};
use zerocopy::{FromBytes, Immutable, IntoBytes};

pub struct PatchManagerPlugin;
impl Plugin for PatchManagerPlugin {
    fn build(&self, app: &mut App) {
        // app.sub_app_mut(RenderApp).add_systems(RenderStartup, init_patch_render_phase);
    }
}

#[repr(C)]
#[derive(IntoBytes, FromBytes, Immutable, Debug, Copy, Clone)]
pub struct SubdivisionContext {
    // Number of unique vertices in a patch in the target subdivision level. e.g. Skip past this
    // many vertices in a buffer to get to the next patch.
    target_stride: u32,

    // The final target subdivision level of the subdivision process.
    target_subdivision_level: u32,
}
assert_eq_size!(SubdivisionContext, [u32; 2]);
assert_eq_align!(SubdivisionContext, [f32; 4]);

#[repr(C)]
#[derive(IntoBytes, FromBytes, Immutable, Debug, Copy, Clone)]
pub struct SubdivisionExpandContext {
    // The target subdivision level after this expand call.
    current_target_subdivision_level: u32,

    // The number of vertices to skip at the start of each patch. This is always the number of
    // vertices in the previous subdivision level.
    skip_vertices_in_patch: u32,

    // The number of vertices to compute per patch in this expand phase. This will always be the
    // number of vertices in this subdivision level *minus* the number of vertices in the previous
    // expansion level.
    compute_vertices_in_patch: u32,
}
assert_eq_size!(SubdivisionExpandContext, [u32; 3]);
assert_eq_align!(SubdivisionExpandContext, [f32; 4]);

pub(crate) struct PatchManager {
    // The frame-coherent optimal patch tree. e.g. the hard part.
    patch_tree: PatchTree,

    // Hot cache for CPU patch and vertex generation. Note that we read from live_patches
    // for patch-winding when doing the draw later.
    desired_patch_count: usize,
    live_patches: Vec<(PatchHandle, PatchWinding)>,
    live_vertices: Vec<TerrainUploadVertex>,
    // CPU generated patch corner vertices. Input to subdivision.
    // patch_upload_buffer: wgpu::Buffer,

    /*
    // Metadata about the subdivision. We upload this in a buffer, then save the uploaded context
    // in our manager as a reference so that the CPU and GPU can share some constant (per run)
    // metadata about the subdivision and buffers that are not hard-coded.
    subdivide_context: SubdivisionContext,

    // Subdivision process.
    subdivide_prepare_pipeline: wgpu::ComputePipeline,
    subdivide_prepare_bind_group: wgpu::BindGroup,
    subdivide_expand_pipeline: wgpu::ComputePipeline,
    subdivide_expand_bind_groups: Vec<(SubdivisionExpandContext, wgpu::BindGroup)>,

    // The final buffer containing the fully tessellated and height-offset vertices.
    target_vertex_count: u32,
    target_vertex_buffer: wgpu::Buffer,
    target_vertex_buffer_size: wgpu::BufferAddress,

    // Index buffers for each patch size and winding.
    wireframe_index_buffers: Vec<wgpu::Buffer>,
    wireframe_index_ranges: Vec<Range<u32>>,
    tristrip_index_buffer: wgpu::Buffer,
    tristrip_index_ranges: Vec<Range<u32>>,

    // Indirect command builder and target buffer
    indirect_commands: Vec<DrawIndexedIndirect>,
    indirect_buffer: wgpu::Buffer,
     */
}

// impl ExtractResource for PatchManager {
//     type Source = ();
//
//     fn extract_resource(source: &Self::Source) -> Self {
//         todo!()
//     }
// }

impl fmt::Debug for PatchManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PatchManger")
    }
}

fn make_prepare_bind_group(
    mut commands: Commands,
    terrain: Res<Terrain>,
    // gpu_images: Res<RenderAssets<GpuImage>>,
    // game_of_life_images: Res<GameOfLifeImages>,
    // game_of_life_uniforms: Res<GameOfLifeUniforms>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    // queue: Res<RenderQueue>,
) {
    println!("RUNNING MAKE PREPARE BIND GROUP");

    // let view_a = gpu_images.get(&game_of_life_images.texture_a).unwrap();
    // let view_b = gpu_images.get(&game_of_life_images.texture_b).unwrap();
    //
    // // Uniform buffer is used here to demonstrate how to set up a uniform in a compute shader
    // // Alternatives such as storage buffers or push constants may be more suitable for your use case
    // let mut uniform_buffer = UniformBuffer::from(game_of_life_uniforms.into_inner());
    // uniform_buffer.write_buffer(&render_device, &queue);
    //
    // let bind_group_0 = render_device.create_bind_group(
    //     Some("terrain-subdivide-prepare"),
    //     &pipeline_cache.get_bind_group_layout(&pipeline.texture_bind_group_layout),
    //     &BindGroupEntries::sequential((
    //         &view_a.texture_view,
    //         &view_b.texture_view,
    //         &uniform_buffer,
    //     )),
    // );
    // let bind_group_1 = render_device.create_bind_group(
    //     None,
    //     &pipeline_cache.get_bind_group_layout(&pipeline.texture_bind_group_layout),
    //     &BindGroupEntries::sequential((
    //         &view_b.texture_view,
    //         &view_a.texture_view,
    //         &uniform_buffer,
    //     )),
    // );
    // commands.insert_resource(GameOfLifeImageBindGroups([bind_group_0, bind_group_1]));
}

impl PatchManager {
    pub fn new(
        app: &mut App,
        max_level: usize,
        target_refinement: f64,
        desired_patch_count: usize,
        max_subdivisions: usize,
        // gpu: &Gpu,
    ) -> Result<Self> {
        let patch_tree = PatchTree::new(max_level, target_refinement, desired_patch_count);
        let live_patches = Vec::with_capacity(desired_patch_count);
        let live_vertices = Vec::with_capacity(3 * desired_patch_count);

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            make_prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            // prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );
        // app.add_plugins(ExtractResourcePlugin::<PatchManager>::default());
        /*
        // Extract the game of life image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app.add_plugins(ExtractResourcePlugin::<GameOfLifeImages>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(GameOfLifeLabel, GameOfLifeNode::default());
        render_graph.add_node_edge(GameOfLifeLabel, bevy::render::graph::CameraDriverLabel);
         */

        /*
        let patch_upload_stride = 3; // 3 vertices per patch in the upload buffer.
        let patch_upload_byte_size = TerrainUploadVertex::mem_size() * patch_upload_stride;
        let patch_upload_buffer_size =
            (patch_upload_byte_size * desired_patch_count) as wgpu::BufferAddress;
        let patch_upload_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain-geo-patch-vertex-buffer"),
            size: patch_upload_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create the context buffer for uploading uniform data to our subdivision process.
        let subdivide_context = SubdivisionContext {
            target_stride: Self::vertices_per_subdivision(max_subdivisions) as u32,
            target_subdivision_level: max_subdivisions as u32,
        };
        let subdivide_context_buffer = gpu.push_data(
            "subdivision-context",
            &subdivide_context,
            wgpu::BufferUsages::UNIFORM,
        );

        // Create target vertex buffer.
        let target_vertex_count = subdivide_context.target_stride * desired_patch_count as u32;
        let target_patch_byte_size =
            TerrainVertex::mem_size(0) * subdivide_context.target_stride as usize;
        assert_eq!(target_patch_byte_size % 4, 0);
        let target_vertex_buffer_size =
            (target_patch_byte_size * desired_patch_count) as wgpu::BufferAddress;
        let target_vertex_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain-geo-sub-vertex-buffer"),
            size: target_vertex_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let (subdivide_prepare_pipeline, mut layouts) =
            ComputePipelineBuilder::new("terrain-subdivide-prepare", 1, gpu.device())
                .with_shader_wgsl(
                    "subdivide_prepare.wgsl",
                    include_str!("../../target/subdivide_prepare.wgsl"),
                )
                .with_binding_entries(
                    0,
                    vec![
                        comp::uniform::<SubdivisionContext>(0),
                        comp::buffer_rw_for(1, &target_vertex_buffer),
                        comp::buffer_for(2, &patch_upload_buffer),
                    ],
                )
                .build();
        let subdivide_prepare_bind_group_layout = layouts.remove(0);
        let subdivide_prepare_bind_group =
            gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("terrain-geo-subdivide-bind-group"),
                layout: &subdivide_prepare_bind_group_layout,
                entries: &[
                    binding::buffer(0, &subdivide_context_buffer),
                    binding::buffer(1, &target_vertex_buffer),
                    binding::buffer(2, &patch_upload_buffer),
                ],
            });

        // Create the index dependence lut.
        let index_dependency_lut_buffer = gpu.push_slice(
            "terrain-geo-index-dependency-lut",
            get_index_dependency_lut(max_subdivisions),
            wgpu::BufferUsages::STORAGE,
        );

        let (subdivide_expand_pipeline, mut layouts) =
            ComputePipelineBuilder::new("terrain-subdivide-expand", 1, gpu.device())
                .with_shader_wgsl(
                    "subdivide_expand.wgsl",
                    include_str!("../../target/subdivide_expand.wgsl"),
                )
                .with_binding_entries(
                    0,
                    vec![
                        comp::uniform::<SubdivisionContext>(0),
                        comp::uniform::<SubdivisionExpandContext>(1),
                        comp::buffer_rw_for(2, &target_vertex_buffer),
                        comp::buffer_for(3, &index_dependency_lut_buffer),
                    ],
                )
                .build();
        let subdivide_expand_bind_group_layout = layouts.remove(0);

        let mut subdivide_expand_bind_groups = Vec::new();
        for i in 1..max_subdivisions + 1 {
            let expand_context = SubdivisionExpandContext {
                current_target_subdivision_level: i as u32,
                skip_vertices_in_patch: Self::vertices_per_subdivision(i - 1) as u32,
                compute_vertices_in_patch: (Self::vertices_per_subdivision(i)
                    - Self::vertices_per_subdivision(i - 1))
                    as u32,
            };
            let expand_context_buffer = gpu.push_data(
                "terrain-geo-expand-context-SUB",
                &expand_context,
                wgpu::BufferUsages::UNIFORM,
            );
            let subdivide_expand_bind_group =
                gpu.device().create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("terrain-geo-subdivide-expand-bind-group"),
                    layout: &subdivide_expand_bind_group_layout,
                    entries: &[
                        binding::buffer(0, &subdivide_context_buffer),
                        binding::buffer(1, &expand_context_buffer),
                        binding::buffer(2, &target_vertex_buffer),
                        binding::buffer(3, &index_dependency_lut_buffer),
                    ],
                });
            subdivide_expand_bind_groups.push((expand_context, subdivide_expand_bind_group));
        }

        // Create each of the 4 wireframe index buffers at this subdivision level.
        let wireframe_index_buffers = PatchWinding::all_windings()
            .iter()
            .map(|&winding| {
                gpu.push_slice(
                    "terrain-geo-wireframe-indices-SUB",
                    get_wireframe_index_buffer(max_subdivisions, winding),
                    wgpu::BufferUsages::INDEX,
                )
            })
            .collect::<Vec<_>>();

        let wireframe_index_ranges = PatchWinding::all_windings()
            .iter()
            .map(|&winding| {
                0u32..get_wireframe_index_buffer(max_subdivisions, winding).len() as u32
            })
            .collect::<Vec<_>>();

        // Create each of the 4 tristrip index buffers at this subdivision level.
        let tristrip_index_buffer = gpu.push_slice(
            "terrain-geo-tristrip-indices",
            get_tri_strip_indices(),
            wgpu::BufferUsages::INDEX,
        );

        let tristrip_index_ranges = PatchWinding::all_windings()
            .iter()
            .map(|&winding| get_tri_strip_index_range(max_subdivisions, winding))
            .collect::<Vec<_>>();

        let indirect_commands = Vec::with_capacity(desired_patch_count);
        let indirect_buffer_size =
            (mem::size_of::<DrawIndexedIndirect>() * desired_patch_count) as wgpu::BufferAddress;
        let indirect_buffer = gpu.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain-geo-indirect-buffer"),
            size: indirect_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        */

        Ok(PatchManager {
            patch_tree,
            desired_patch_count,
            live_patches,
            live_vertices,
            // patch_upload_buffer,
            // subdivide_context,
            // subdivide_prepare_pipeline,
            // subdivide_prepare_bind_group,
            // subdivide_expand_pipeline,
            // subdivide_expand_bind_groups,
            // target_vertex_count,
            // target_vertex_buffer,
            // target_vertex_buffer_size,
            // wireframe_index_buffers,
            // wireframe_index_ranges,
            // tristrip_index_buffer,
            // tristrip_index_ranges,
            // indirect_commands,
            // indirect_buffer,
        })
    }

    pub(crate) fn toggle_show_culling(&mut self) {
        self.patch_tree.toggle_show_culling();
    }

    // Detect when a patch crosses a seam and re-order the graticules so that it overlaps,
    // preventing what will become texture coordinates from going backwards.
    fn relap_for_seam(
        lon0: &mut Angle<Radians>,
        lon1: &mut Angle<Radians>,
        lon2: &mut Angle<Radians>,
    ) {
        const LIM: f64 = FRAC_PI_2;
        if *lon0 > radians!(LIM) && (*lon1 < radians!(-LIM) || *lon2 < radians!(-LIM))
            || *lon1 > radians!(LIM) && (*lon0 < radians!(-LIM) || *lon2 < radians!(-LIM))
            || *lon2 > radians!(LIM) && (*lon0 < radians!(-LIM) || *lon1 < radians!(-LIM))
        {
            if lon0.sign() < 0 && *lon0 < radians!(-1e-6) {
                *lon0 += degrees!(360);
            }
            if lon1.sign() < 0 && *lon1 < radians!(-1e-6) {
                *lon1 += degrees!(360);
            }
            if lon2.sign() < 0 && *lon2 < radians!(-1e-6) {
                *lon2 += degrees!(360);
            }
        }
    }

    pub fn track_state_changes(
        &mut self,
        camera: &OptimizeCamera,
        optimize_camera: &OptimizeCamera,
        gizmos: &mut Option<&mut Gizmos>,
    ) {
        // Select optimal live patches from our coherent patch tree.
        self.live_patches.clear();
        self.patch_tree
            .optimize_for_view(optimize_camera, &mut self.live_patches, gizmos);
        assert!(self.live_patches.len() <= self.desired_patch_count);

        // Build CPU vertices for upload. Make sure to track visibility for our tile loader.
        self.live_vertices.clear();
        /*
        self.indirect_commands.clear();
        let view = camera.world_to_eye_m();
        for (offset, (i, winding)) in self.live_patches.iter().enumerate() {
            if offset >= self.desired_patch_count {
                continue;
            }
            let patch = self.patch_tree.get_patch(*i);

            let draw_range = self.tristrip_index_range(*winding);
            let base_vertex = self.patch_vertex_buffer_offset(offset as i32);
            self.indirect_commands.push(DrawIndexedIndirect {
                index_count: draw_range.end - draw_range.start,
                instance_count: 1,
                base_index: draw_range.start,
                vertex_offset: base_vertex,
                base_instance: 0,
            });

            // Points in geocenter KM f64 for precision reasons.
            let [pw0, pw1, pw2] = patch.points();

            // Compute normals and rotate from world to eye
            let nv0_2 = view.transform_vector3(pw0.dvec3().normalize());
            let nv1_2 = view.transform_vector3(pw1.dvec3().normalize());
            let nv2_2 = view.transform_vector3(pw2.dvec3().normalize());

            // Move verts from global coordinates into eye space.
            // Note: f64 is absolutely required for precision up to this point.
            let pv0 = *view * *pw0;
            let pv1 = *view * *pw1;
            let pv2 = *view * *pw2;

            // Convert from geocenter f64 kilometers into graticules.
            let mut g0 = Geodetic::from(*pw0);
            let mut g1 = Geodetic::from(*pw1);
            let mut g2 = Geodetic::from(*pw2);
            Self::relap_for_seam(g0.lon_mut(), g1.lon_mut(), g2.lon_mut());

            g0.set_geocenter(meters!(kilometers!(*EARTH_RADIUS).f64()));
            g1.set_geocenter(meters!(kilometers!(*EARTH_RADIUS).f64()));
            g2.set_geocenter(meters!(kilometers!(*EARTH_RADIUS).f64()));

            let tuv0 = TerrainUploadVertex::new(&pv0, &nv0_2, &g0);
            let tuv1 = TerrainUploadVertex::new(&pv1, &nv1_2, &g1);
            let tuv2 = TerrainUploadVertex::new(&pv2, &nv2_2, &g2);

            self.live_vertices.push(tuv0);
            self.live_vertices.push(tuv1);
            self.live_vertices.push(tuv2);
        }
        while self.indirect_commands.len() < self.desired_patch_count {
            self.indirect_commands.push(DrawIndexedIndirect::default());
        }
        while self.live_vertices.len() < 3 * self.desired_patch_count {
            self.live_vertices.push(TerrainUploadVertex::empty());
        }
         */
    }

    /*
    pub fn encode_uploads(&self, gpu: &Gpu, encoder: &mut wgpu::CommandEncoder) {
        // Upload the raw live vertices buffer in a single block for dispersal by "prepare".
        gpu.upload_slice_to(&self.live_vertices, &self.patch_upload_buffer, encoder);

        // Upload the indirect buffer for drawing the eventual patches
        gpu.upload_slice_to(&self.indirect_commands, &self.indirect_buffer, encoder)
    }
     */

    /*
    pub fn tessellate(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("terrain-tessellate-compute-pass"),
            timestamp_writes: None,
        });

        // Copy our upload buffer into seed positions for subdivisions.
        let patch_count = 3 * self.desired_patch_count as u32;
        assert!(patch_count < u16::MAX as u32);
        cpass.set_pipeline(&self.subdivide_prepare_pipeline);
        cpass.set_bind_group(0, &self.subdivide_prepare_bind_group, &[]);
        cpass.dispatch_workgroups(patch_count, 1, 1);

        // Iterative subdivision by recursion level
        const WORKGROUP_WIDTH: u32 = 65536;
        cpass.set_pipeline(&self.subdivide_expand_pipeline);
        for i in 0usize..self.target_patch_subdivision_level() as usize {
            let (expand, bind_group) = &self.subdivide_expand_bind_groups[i];
            let iteration_count =
                expand.compute_vertices_in_patch * self.desired_patch_count as u32;
            let wg_x = (iteration_count % WORKGROUP_WIDTH).max(1);
            let wg_y = (iteration_count / WORKGROUP_WIDTH).max(1);
            cpass.set_bind_group(0, bind_group, &[]);
            cpass.dispatch_workgroups(wg_x, wg_y, 1);
        }
    }
     */

    fn vertices_per_subdivision(d: usize) -> usize {
        (((2f64.powf(d as f64) + 1.0) * (2f64.powf(d as f64) + 2.0)) / 2.0).floor() as usize
    }

    // pub(crate) fn target_vertex_count(&self) -> u32 {
    //     self.target_vertex_count
    // }

    pub(crate) fn patch_winding(&self, patch_number: i32) -> PatchWinding {
        assert!(patch_number >= 0);
        if patch_number < self.live_patches.len() as i32 {
            self.live_patches[patch_number as usize].1
        } else {
            PatchWinding::Full
        }
    }

    /*
    #[inline]
    pub fn patch_vertex_buffer_offset(&self, patch_number: i32) -> i32 {
        debug_assert!(patch_number >= 0);
        (patch_number as u32 * self.subdivide_context.target_stride) as i32
    }

    #[inline]
    pub fn target_patch_subdivision_level(&self) -> u32 {
        self.subdivide_context.target_subdivision_level
    }
     */

    pub fn num_patches(&self) -> i32 {
        self.live_patches.len() as i32
    }

    /*
    pub(crate) fn vertex_buffer(&self) -> wgpu::BufferSlice {
        self.target_vertex_buffer.slice(..)
    }

    pub(crate) fn target_vertex_buffer(&self) -> &wgpu::Buffer {
        &self.target_vertex_buffer
    }

    pub(crate) fn target_vertex_buffer_size(&self) -> wgpu::BufferAddress {
        self.target_vertex_buffer_size
    }

    pub fn wireframe_index_buffer(&self, winding: PatchWinding) -> wgpu::BufferSlice {
        self.wireframe_index_buffers[winding.index()].slice(..)
    }

    pub fn wireframe_index_range(&self, winding: PatchWinding) -> Range<u32> {
        self.wireframe_index_ranges[winding.index()].clone()
    }

    pub fn tristrip_index_buffer(&self) -> wgpu::BufferSlice {
        self.tristrip_index_buffer.slice(..)
    }

    pub fn tristrip_index_range(&self, winding: PatchWinding) -> Range<u32> {
        self.tristrip_index_ranges[winding.index()].clone()
    }

    #[allow(unused)]
    pub fn draw_indirect_commands(&self) -> &[DrawIndexedIndirect] {
        &self.indirect_commands
    }

    #[allow(unused)]
    pub fn draw_indirect_buffer(&self) -> &wgpu::Buffer {
        &self.indirect_buffer
    }
     */
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_subdivision_vertex_counts() {
        let expect = [3, 6, 15, 45, 153, 561, 2145, 8385];
        for (i, &value) in expect.iter().enumerate() {
            assert_eq!(value, PatchManager::vertices_per_subdivision(i));
        }
    }
}
