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
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use geodesy::Geodetic;

/// All systems that run on behalf of cameras, so that other systems can ensure correct ordering.
#[derive(SystemSet, Clone, Debug, Eq, PartialEq, Hash)]
pub enum GeoCameraSet {
    TranscribePositions,
}

/// Add a Geodetic-centric camera to the world named Camera
pub struct GeoCameraPlugin {
    initial: Geodetic,
}

impl Default for GeoCameraPlugin {
    fn default() -> Self {
        Self {
            initial: Geodetic::new(radians!(0), radians!(0), meters!(0)),
        }
    }
}

impl GeoCameraPlugin {
    pub fn with_initial_position(mut self, initial: Geodetic) -> Self {
        self.initial = initial;
        self
    }
}

impl Plugin for GeoCameraPlugin {
    fn build(&self, app: &mut App) {
        let position = self.initial;
        app.add_systems(Startup, move |mut commands: Commands| {
            // The one and only camera entity
            commands.spawn((
                Name::new("Camera"),
                GeoCamera { position },
                Camera3d::default(),
                Tonemapping::AgX,
                Projection::Perspective(PerspectiveProjection {
                    fov: 70f32.to_radians(),
                    near: 0.1,
                    far: 10.0,
                    ..default()
                }),
                Transform::default(),
            ));
        });

        // Note: rendering is eye-relative, so it's not clear where we make use of the cartesian
        // and if that shouldn't just be on-demand.
    }
}

#[derive(Component)]
pub struct GeoCamera {
    position: Geodetic,
}

impl GeoCamera {
    pub fn position(&self) -> &Geodetic {
        &self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let pos = Geodetic::new(radians!(0), radians!(1), meters!(42));
        let mut app = App::new().add_plugins((
            MinimalPlugins,
            GeoCameraPlugin::default().with_initial_position(pos),
        ));
        app.update();
        let cam = app.world_mut().query::<&GeoCamera>().single();
        assert!(cam.is_ok());
        assert_eq!(cam.unwrap().position.asl().f64(), pos.asl().f64());
    }
}
