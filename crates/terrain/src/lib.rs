mod common;
mod patch;
mod tables;

use crate::patch::PatchManagerPlugin;
use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{binding_types::texture_storage_2d, *},
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
    },
};
use clap::Parser;
use std::borrow::Cow;

#[derive(Resource)]
pub struct Terrain {
    // patch_manager: PatchManager,
}

#[derive(Parser)]
pub struct TerrainPlugin {
    geo_max_cpu_level: i32,
    geo_target_refinement: f64,
    geo_desired_patch_count: i32,
    geo_gpu_subdivisions: i32,
    tile_max_level: i32,
    tile_target_refinement: f64,
    tile_desired_patch_count: i32,
    tile_cache_size: i32,
}

impl Default for TerrainPlugin {
    // TODO: figure out our configuration and arguments story
    fn default() -> Self {
        Self {
            geo_max_cpu_level: 16,
            geo_target_refinement: 0.03,
            geo_desired_patch_count: 400,
            geo_gpu_subdivisions: 6,
            tile_max_level: 10,
            tile_target_refinement: 0.03,
            tile_desired_patch_count: 200,
            tile_cache_size: 768,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct TerrainLabel;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {

        app.add_plugins((PatchManagerPlugin,));

        // let patch_manager = PatchManager::new(
        //     app,
        //     geo_max_cpu_level,
        //     geo_target_refinement,
        //     geo_desired_patch_count,
        //     geo_gpu_subdivisions,
        // ).expect("patch manager");
        // app.insert_resource(Terrain {
        //     patch_manager
        // });

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
    }

    // fn finish(&self, app: &mut App) {
    //     let render_app = app.sub_app_mut(RenderApp);
    //     render_app.init_resource::<GameOfLifePipeline>();
    // }
}
