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
use crate::geodb::{
    GeoDb,
    // SelectedTarmacHeight
};
use bevy::prelude::*;
use clap::Parser;
use runtime::StdPaths;

#[derive(SystemSet, Clone, Debug, Eq, PartialEq, Hash)]
pub enum GeoDbStep {
    CheckDownloads,
    UpdateTarmacHeight,
}

#[derive(Clone, Debug, Default, Parser)]
pub struct GeoDbPlugin {
    /// Override the terrain height map.
    #[arg(long)]
    tiles_height_url: Option<String>,

    /// Override the terrain color map.
    #[arg(long)]
    tiles_color_url: Option<String>,
}

impl GeoDbPlugin {
    const COLORS_URL: &'static str =
        "https://storage.googleapis.com/openfa/bmng-jan-sparse-512-lzma.cog";
    const HEIGHTS_URL: &'static str =
        "https://storage.googleapis.com/openfa/srtm-sparse-512-lzma.cog";

    pub fn tiles_color_url(&self) -> &str {
        if let Some(ref url) = self.tiles_color_url {
            url
        } else {
            Self::COLORS_URL
        }
    }

    pub fn tiles_height_url(&self) -> &str {
        if let Some(ref url) = self.tiles_height_url {
            url
        } else {
            Self::HEIGHTS_URL
        }
    }
}

impl Plugin for GeoDbPlugin {
    fn build(&self, app: &mut App) {
        let height_cog_url = self.tiles_height_url().to_string();
        let color_cog_url = self.tiles_color_url().to_string();
        app.add_systems(Startup, move |world: &mut World| {
            connect_to_geodb(&height_cog_url, &color_cog_url, world)
        });
        app.add_systems(
            FixedPreUpdate,
            GeoDb::sys_check_downloads.in_set(GeoDbStep::CheckDownloads),
        );
    }
}

fn connect_to_geodb(height_cog_url: &str, color_cog_url: &str, world: &mut World) -> Result<()> {
    let geodb = GeoDb::new(
        world.resource::<StdPaths>().state_dir(),
        height_cog_url,
        color_cog_url,
    )?;
    world.insert_resource(geodb);
    // runtime.add_sim_system(
    //     Self::sys_check_downloads
    //         .pipe(report_errors)
    //         .in_set(GeoDbStep::CheckDownloads),
    // );
    // runtime.add_sim_system(
    //     Self::sys_apply_tarmac_height
    //         .pipe(report_errors)
    //         .in_set(GeoDbStep::UpdateTarmacHeight)
    //         .after(GeoDbStep::CheckDownloads),
    // );
    // runtime.register_event::<SelectedTarmacHeight>();
    Ok(())
}
