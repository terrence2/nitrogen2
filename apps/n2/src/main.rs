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
use anyhow::Result;
use bevy::{
    // asset::embedded_asset,
    diagnostic::FrameTimeDiagnosticsPlugin,
    ecs::schedule::{LogLevel, ScheduleBuildSettings},
    prelude::*,
    window::{Window, WindowMode, WindowResolution},
};
// use bevy_atmosphere::prelude::*;
// use bevy_common_assets::ron::RonAssetPlugin;
use bevy_egui::{EguiContexts, EguiPlugin};
use bevy_rapier3d::prelude::*;
use clap::Parser;
// use game_state::{ConfigurationPlugin, GameStatePlugin};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Turn on system ordering debugging.
    #[arg(short, long, default_value = "true")]
    debug_order: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut app = App::new();
    if cli.debug_order {
        app.edit_schedule(Startup, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Warn,
                ..default()
            });
        })
            .edit_schedule(PreUpdate, |schedule| {
                schedule.set_build_settings(ScheduleBuildSettings {
                    ambiguity_detection: LogLevel::Warn,
                    ..default()
                });
            })
            .edit_schedule(Update, |schedule| {
                schedule.set_build_settings(ScheduleBuildSettings {
                    ambiguity_detection: LogLevel::Warn,
                    ..default()
                });
            })
            .edit_schedule(PostUpdate, |schedule| {
                schedule.set_build_settings(ScheduleBuildSettings {
                    ambiguity_detection: LogLevel::Warn,
                    ..default()
                });
            });
    }
    app.add_plugins((
        (
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    resolution: WindowResolution::new(1920., 1080.),
                    ..default()
                }),
                ..default()
            }),
            // .set(AssetPlugin {
            //     file_path: "assets/art".into(),
            //     processed_file_path: "pkg/art".into(),
            //     ..default()
            // }),
            FrameTimeDiagnosticsPlugin::default(),
            bevy_framepace::FramepacePlugin, // reduces input lag
            EguiPlugin {
                enable_multipass_for_primary_context: false,
            },
            // AtmospherePlugin,
            MeshPickingPlugin,
            RapierPhysicsPlugin::<NoUserData>::default().in_fixed_schedule(),
            RapierDebugRenderPlugin {
                enabled: false,
                ..default()
            },
            // RonAssetPlugin::<PuzzleDefinition>::new(&["definition.ron"]),
        ),
    ))
        .register_type::<Transform>()
        .register_type::<Visibility>()
        .add_systems(Startup, do_setup)
        .add_systems(Update, do_input);

    app.run();

    Ok(())
}

fn do_setup(mut contexts: EguiContexts) {
    egui_extras::install_image_loaders(contexts.ctx_mut());
}

fn do_input(keyboard: Res<ButtonInput<KeyCode>>, mut app_exit: EventWriter<AppExit>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        app_exit.write(AppExit::Success);
    }
}
