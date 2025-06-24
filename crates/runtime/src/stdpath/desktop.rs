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
// use crate::{Extension, Runtime};
// use anyhow::anyhow;
// use nitrous::{inject_nitrous_resource, NitrousResource};
use bevy::prelude::*;
use platform_dirs::AppDirs;
use std::{fs::create_dir_all, path::Path};

#[derive(Debug)]
pub struct StdPathsPlugin {
    name: String,
}

impl StdPathsPlugin {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
        }
    }
}

impl Plugin for StdPathsPlugin {
    fn build(&self, app: &mut App) {
        let stdpaths = StdPaths {
            app_dirs: AppDirs::new(Some(&self.name), false).expect("Failed to find app dirs"),
        };

        create_dir_all(&stdpaths.app_dirs.config_dir).unwrap_or_else(|err| {
            panic!(
                "failed to create config dir {:?}: {err}",
                &stdpaths.app_dirs.config_dir
            )
        });
        create_dir_all(&stdpaths.app_dirs.state_dir).unwrap_or_else(|err| {
            panic!(
                "failed to create state dir {:?}: {err}",
                &stdpaths.app_dirs.state_dir
            )
        });

        app.insert_resource(stdpaths);
    }
}

#[derive(Resource)]
pub struct StdPaths {
    app_dirs: AppDirs,
}

impl StdPaths {
    pub fn state_dir(&self) -> Option<&Path> {
        Some(&self.app_dirs.state_dir)
    }
}
