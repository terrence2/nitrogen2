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
use crate::{GeoDb, MapKind, MapName};
use absolute_unit::prelude::*;
use bevy_ecs::prelude::*;
use geodesy::GeodeBB;
use nitrous::{inject_nitrous_component, NitrousComponent};
use std::collections::HashSet;

/// Attach a flattener to an entity to flatten a patch of ground.
#[derive(NitrousComponent)]
#[component(name = "flatten")]
pub struct Flatten {
    // Pre-compute the intersecting maps so that we don't have to do an expensive `sample` operation
    // just to find out that a newly loaded map isn't even in the same hemisphere.
    intersecting_maps: Option<HashSet<MapName>>,

    // We need to pick a level on our first load so that we don't have to revisit pixels
    // when subsequent loads happen.
    target_height: Option<Length<Meters>>,

    // We need to track the bounds both to get the intersecting maps (once metadata is available)
    // and to see if any particular sample should be flattened.
    bounds: GeodeBB,
}

#[inject_nitrous_component]
impl Flatten {
    pub fn new(bounds: &GeodeBB) -> Self {
        Self {
            intersecting_maps: None,
            target_height: None,
            bounds: bounds.clone(),
        }
    }

    pub fn bounds(&self) -> &GeodeBB {
        &self.bounds
    }

    pub fn target_height(&self) -> Option<Length<Meters>> {
        self.target_height
    }

    pub fn set_target_height(&mut self, height: Length<Meters>) {
        self.target_height = Some(height);
    }

    pub fn has_cached_intersections(&self) -> bool {
        self.intersecting_maps.is_some()
    }

    pub fn cache_intersections(&mut self, geodb: &GeoDb) {
        assert!(self.intersecting_maps.is_none());
        self.intersecting_maps = geodb.compute_intersecting_maps(MapKind::Heights, &self.bounds);
    }

    pub fn intersects_with_map_load(&self, name: &MapName) -> bool {
        assert!(self.intersecting_maps.is_some());
        self.intersecting_maps.as_ref().unwrap().contains(name)
    }
}
