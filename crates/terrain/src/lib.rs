mod common;
mod patch;
mod tables;

use crate::patch::PatchManager;
use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::{RenderAssetUsages, RenderAssets},
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
    patch_manager: PatchManager,
}

#[derive(Parser)]
pub struct TerrainPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct TerrainLabel;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        // TODO: figure out our configuration and arguments story
        let (
            geo_max_cpu_level,
            geo_target_refinement,
            geo_desired_patch_count,
            tile_max_level,
            tile_target_refinement,
            tile_desired_patch_count,
        ) = (16, 0.03, 400, 10, 0.03, 200);
        let (geo_gpu_subdivisions, tile_cache_size) = (6, 768);

        app.insert_resource(Terrain {
            patch_manager: PatchManager::new(
                geo_max_cpu_level,
                geo_target_refinement,
                geo_desired_patch_count,
                geo_gpu_subdivisions,
            )
            .unwrap(),
        });

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
