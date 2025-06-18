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
use crate::{levels::OverviewLevel, GeoDb, MapKind, MapName};
use absolute_unit::prelude::*;
use bevy_ecs::prelude::*;
use geodesy::GeodeBB;
use nitrous::{inject_nitrous_component, NitrousComponent};
use runtime::RuntimeResource;
use std::collections::HashSet;

/// Attach a foundation to an entity to affix it to the ground.
/// GeoDB uses these to make sure that any foundations inside a
/// recently updated patch of heightmap stay on the ground.
#[derive(NitrousComponent)]
#[component(name = "foundation")]
pub struct Foundation {
    // Pre-compute the intersecting maps so that we don't have to do an expensive `sample` operation
    // just to find out that this load isn't even in the same hemisphere.
    intersecting_maps: Option<HashSet<MapName>>,

    // We need to track the bounds both to get the intersecting maps (once metadata is available)
    // and to see if any particular sample actually impacts this foundation.
    bounds: GeodeBB,

    // We only get an absolution position from the new height map, so store the prior height to
    // avoid having to do two lookups, or worse, to get the prior height so that we can adjust.
    // Note that we can't just use the Frame's height since the frame may be accounting for
    // positioning of a shape independent from ground leveling.
    offset_to_ground: Length<Meters>,

    // We only want to refine the position when the level becomes more refined.
    current_refinement: Option<OverviewLevel>,
    last_refined_at_step: (u64, u64),
}

#[inject_nitrous_component]
impl Foundation {
    pub fn new(offset_to_ground: Length<Meters>, bounds: &GeodeBB) -> Self {
        let mut bounds = bounds.to_owned();
        bounds.expand_with_border(meters!(100));
        Self {
            intersecting_maps: None,
            bounds,
            offset_to_ground,
            current_refinement: None,
            last_refined_at_step: (0, 0),
        }
    }

    pub fn offset_to_ground(&self) -> Length<Meters> {
        self.offset_to_ground
    }

    pub fn bounds(&self) -> &GeodeBB {
        &self.bounds
    }

    pub fn improves_refinement(&self, name: &MapName) -> bool {
        if let Some(level) = self.current_refinement {
            name.level().is_more_refined_than(level)
        } else {
            true
        }
    }

    pub fn last_refined_at_step(&self) -> (u64, u64) {
        self.last_refined_at_step
    }

    pub(crate) fn note_refinement(&mut self, name: &MapName, rt: &RuntimeResource) {
        debug_assert!(self.improves_refinement(name));
        self.current_refinement = Some(name.level());
        self.last_refined_at_step = rt.state();
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
