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
    geotiff::{GeoTiff, LoadState, MapRef, SampleGrid},
    levels::{OverviewLevel, TileSubdivisionLevel},
    lru::{Lru, MapState},
    options::GeoDbOpts,
    Flatten, Foundation, GroundStabilizer, Tarmac, TarmacBuilding,
};
use absolute_unit::prelude::*;
use anyhow::{ensure, Result};
use bevy_ecs::prelude::*;
use geodesy::{Geode, GeodeBB, GeodeticBB};
use nitrous::{inject_nitrous_resource, method, NitrousResource};
use phase::{CollisionImpostor, Frame};
use runtime::{report_errors, Extension, Runtime, RuntimeResource, StdPaths};
use smallvec::SmallVec;
use std::{collections::HashSet, fmt, mem, path::Path};

/// The GeoDB has 3 layers:
///   1) In-Memory uncompressed LRU cache
///   2) On-Disk permanent compressed cache
///   3) Network/File access

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[repr(u8)]
pub enum MapKind {
    Heights = 0,
    Colors = 1,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[repr(packed)]
pub struct MapName {
    kind: MapKind,
    level: OverviewLevel,
    position: MapRef,
}

impl fmt::Display for MapName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let k = match self.kind {
            MapKind::Heights => "H",
            MapKind::Colors => "C",
        };
        write!(
            f,
            "N:{}:{:02}:{:06}",
            k,
            self.level().offset(),
            self.position()
        )
    }
}

impl MapName {
    #[inline]
    pub(crate) fn new(kind: MapKind, level: OverviewLevel, position: MapRef) -> Self {
        assert_eq!(mem::size_of::<Self>(), 8);
        Self {
            kind,
            level,
            position,
        }
    }

    #[inline]
    pub(crate) fn tiff_index_index(&self) -> (MapKind, OverviewLevel) {
        (self.kind, self.level)
    }

    #[inline]
    pub fn kind(&self) -> MapKind {
        self.kind
    }

    #[inline]
    pub fn level(&self) -> OverviewLevel {
        self.level
    }

    #[inline]
    pub fn position(&self) -> MapRef {
        self.position
    }
}

#[derive(Event, Clone, Copy, Debug)]
pub struct SelectedTarmacHeight {
    // The Id of the tarmac to look up to get the rest of the details.
    tarmac_id: Entity,
    asl: Length<Meters>,
}

#[derive(SystemSet, Clone, Debug, Eq, PartialEq, Hash)]
pub enum GeoDbStep {
    CheckDownloads,
    UpdateTarmacHeight,
}

#[derive(Debug, NitrousResource)]
pub struct GeoDb {
    lru: Lru,
    heights: GeoTiff,
    colors: GeoTiff,
}

impl Extension for GeoDb {
    type Opts = GeoDbOpts;

    fn init(runtime: &mut Runtime, opts: GeoDbOpts) -> Result<()> {
        let height_cog_url = opts.tiles_height_url().to_string();
        let color_cog_url = opts.tiles_color_url().to_string();
        let tm = GeoDb::new(
            runtime.resource::<StdPaths>().state_dir(),
            &height_cog_url,
            &color_cog_url,
        )?;
        runtime.inject_resource(tm)?;
        runtime.add_sim_system(
            Self::sys_check_downloads
                .pipe(report_errors)
                .in_set(GeoDbStep::CheckDownloads),
        );
        runtime.add_sim_system(
            Self::sys_apply_tarmac_height
                .pipe(report_errors)
                .in_set(GeoDbStep::UpdateTarmacHeight)
                .after(GeoDbStep::CheckDownloads),
        );
        runtime.register_event::<SelectedTarmacHeight>();
        Ok(())
    }
}

#[inject_nitrous_resource]
impl GeoDb {
    pub fn new(
        cache_dir: Option<&Path>,
        height_cog_url: &str,
        color_cog_url: &str,
    ) -> Result<Self> {
        // FIXME: handle failure to read a tileset?
        let heights = GeoTiff::new(cache_dir, height_cog_url, MapKind::Heights)?;
        let colors = GeoTiff::new(cache_dir, color_cog_url, MapKind::Colors)?;
        Ok(GeoDb {
            heights,
            colors,
            lru: Lru::new()?,
        })
    }

    pub fn tiff(&self, kind: MapKind) -> &GeoTiff {
        match kind {
            MapKind::Heights => &self.heights,
            MapKind::Colors => &self.colors,
        }
    }

    fn tiff_mut(&mut self, kind: MapKind) -> &mut GeoTiff {
        match kind {
            MapKind::Heights => &mut self.heights,
            MapKind::Colors => &mut self.colors,
        }
    }

    /// Synchronously find all tiles that would be required to serve pixels from the given
    /// extents, at the given angular resolution; return their names so that the display
    /// system (or other) can know if they have what they need to fulfil the current view.
    pub fn map_region_to_tiles(
        &mut self,
        bb: &GeodeBB,
        depth: TileSubdivisionLevel,
        kind: MapKind,
        rt: &RuntimeResource,
    ) -> SmallVec<[(MapName, MapState); 9]> {
        if !self.is_ready(kind) {
            return SmallVec::new();
        }
        let overview_level = depth.best_overview_level(kind);
        if overview_level.offset() < self.tiff(kind).num_layers() {
            let maps = self.tiff(kind)[overview_level].find_tiles_in_range(bb);
            self.lru.make_available(&maps, rt)
        } else {
            SmallVec::new()
        }
    }

    pub fn request_load_tile_stack(
        &mut self,
        bb: &GeodeBB,
        min_level: OverviewLevel,
        kind: MapKind,
        rt: &RuntimeResource,
    ) -> Result<()> {
        if !self.is_ready(kind) {
            return Ok(());
        }
        ensure!(min_level.offset() < self.tiff(kind).num_layers());
        for i in min_level.ascending() {
            let level = OverviewLevel::new(i as usize);
            let maps = self.tiff(kind)[level].find_tiles_in_range(bb);
            self.lru.make_available(&maps, rt);
        }
        Ok(())
    }

    fn get_displaced_grid_in_map(&self, map: &MapName, bb: &GeodeBB) -> SampleGrid {
        assert!(self.lru.map_is_ready(map));
        // Sample points that land in the given map.
        let mut grid = self.tiff(MapKind::Heights)[map.level()].sample_expanded_grid(bb, 1);
        // Fill in map points.
        for info in grid.samples_mut() {
            // Note: skipping out-of-map entries allows us to unwrap below because we know that
            //       this map is cached because of the assert above.
            if info.map_name() != map {
                continue;
            }
            debug_assert!(!info.is_oob());
            info.displace(self.lru.get_height_nearest(info).unwrap());
        }
        grid
    }

    // Expect the full grid to be loaded an if not return none. Expects the next
    // level up to recurse through levels.
    fn get_displaced_full_grid_at_level(
        &self,
        bb: &GeodeBB,
        border: usize,
        overview_level: OverviewLevel,
    ) -> Option<SampleGrid> {
        if !self.is_ready(MapKind::Heights) {
            return None;
        }

        // Sample a full, aligned grid that extends past `bb` on each side.
        let mut grid = self.tiff(MapKind::Heights)[overview_level].sample_expanded_grid(bb, border);

        // Sample heights where we have them, return None if there are missing entries.
        for info in grid.samples_mut() {
            let asl = if info.is_oob() {
                meters!(0)
            } else if let Some(asl) = self.lru.get_height_nearest(info) {
                asl
            } else {
                // We found a map that was not loaded, so skip up to the next level.
                return None;
            };
            info.displace(asl);
        }

        Some(grid)
    }

    // Return the smallest SampleGrid fully covering the given bounding box, at the closest
    // resolution to the given precision, or higher if the desired precision is not fully loaded.
    // Note that since this runs in parallel at the level above, we cannot request fetch (requiring
    // mut on self currently) in this loop. If updates to terrain are handled elsewhere, the
    // foundation and position-fixer should fix up if the ground moves under something.
    pub fn get_best_sample_grid(
        &self,
        bb: &GeodeBB,
        min_level: OverviewLevel,
        max_level: OverviewLevel,
    ) -> Option<SampleGrid> {
        // Walk up from `precision` level, up to our topmost overview, trying to find a complete
        // grid at each level. Reset to this top loop if we fail.
        for level in min_level.up_to(&max_level) {
            let overview_level = OverviewLevel::new(level as usize);
            if let Some(grid) = self.get_displaced_full_grid_at_level(bb, 0, overview_level) {
                return Some(grid);
            }
        }

        // Because the top overview is always loaded, this should only happen if the request
        // is outside the equirect map.
        None
    }

    // Do 4-corner interpolation to get an accurate ground height at the best level.
    pub fn get_height_immediate(
        &self,
        grat: &Geode,
        precision: &Length<Meters>,
    ) -> Option<Length<Meters>> {
        if !self.is_ready(MapKind::Heights) {
            return None;
        }

        // Walk up from `precision` level, up to our topmost overview, trying to find a complete
        // grid at each level. Reset to this top loop if we fail.
        let bb = GeodeBB::from_bounds(*grat, *grat);
        let overview_level = OverviewLevel::from_precision(precision);
        for level in overview_level.ascending() {
            let overview_level = OverviewLevel::new(level as usize);
            if let Some(grid) = self.get_displaced_full_grid_at_level(&bb, 0, overview_level) {
                return Some(grid.get_height_at(grat));
            }
        }

        // Because the top overview is always loaded, this should only happen if the request
        // is outside the equirect map.
        None
    }

    pub fn compute_intersecting_maps_at_point(
        &self,
        kind: MapKind,
        geode: &Geode,
    ) -> Option<HashSet<MapName>> {
        self.compute_intersecting_maps(kind, &GeodeBB::from_bounds(*geode, *geode))
    }

    // Find all maps that might intersect the given bounds. Generally one per layer.
    pub fn compute_intersecting_maps(
        &self,
        kind: MapKind,
        bounds: &GeodeBB,
    ) -> Option<HashSet<MapName>> {
        if !self.is_ready(MapKind::Heights) {
            return None;
        }
        let mut out = HashSet::new();
        for level in 0..self.tiff(kind).num_layers() {
            let overview_level = OverviewLevel::new(level);
            // Note: full grid so we even get not-loaded regions
            let grid = self.tiff(kind)[overview_level].sample_expanded_grid(bounds, 1);
            for sample in grid.samples() {
                out.insert(*sample.map_name());
            }
        }
        Some(out)
    }

    // pub fn get_height_nearest(
    //     &self,
    //     grat: &Geode,
    //     precision: &Length<Meters>,
    // ) -> Option<Length<Meters>> {
    //     if !self.is_ready(MapKind::Heights) {
    //         return None;
    //     }
    //     let overview_level = OverviewLevel::from_precision(precision);
    //     for level in overview_level.ascending() {
    //         let overview_level = OverviewLevel::new(level as usize);
    //         if let Some(info) = self.heights[overview_level].sample(grat) {
    //             if let Some(height) = self.lru.get_height_nearest(&info) {
    //                 return Some(height);
    //             }
    //         }
    //     }
    //     None
    // }

    // pub fn get_height_request(
    //     &mut self,
    //     grat: &Geode,
    //     precision: &Length<Meters>,
    //     sim_number: u64,
    // ) -> Option<Length<Meters>> {
    //     if !self.is_ready(MapKind::Heights) {
    //         return None;
    //     }
    //     let overview_level = OverviewLevel::from_precision(precision);
    //     if let Some(info) = self.heights[overview_level].sample(grat) {
    //         self.lru.get_height_request(&info, sim_number)
    //     } else {
    //         None
    //     }
    // }

    fn sys_check_downloads(
        mut geodb: ResMut<GeoDb>,
        rt: Res<RuntimeResource>,
        flattens: Query<(Entity, &mut Flatten, Option<&Tarmac>)>,
        foundations: Query<(&mut Foundation, &mut Frame), Without<GroundStabilizer>>,
        stabs: Query<(&mut GroundStabilizer, &CollisionImpostor, &mut Frame), Without<Foundation>>,
        selected_tarmac_height: EventWriter<SelectedTarmacHeight>,
    ) -> Result<()> {
        geodb.check_downloads(&rt, flattens, foundations, stabs, selected_tarmac_height)?;
        Ok(())
    }

    fn check_downloads(
        &mut self,
        rt: &RuntimeResource,
        flattens: Query<(Entity, &mut Flatten, Option<&Tarmac>)>,
        foundations: Query<(&mut Foundation, &mut Frame), Without<GroundStabilizer>>,
        mut stabs: Query<
            (&mut GroundStabilizer, &CollisionImpostor, &mut Frame),
            Without<Foundation>,
        >,
        selected_tarmac_height: EventWriter<SelectedTarmacHeight>,
    ) -> Result<()> {
        // Grab the current heights of all stabs so we can follow loads accurately
        self.capture_current_stabilizer_heights(&mut stabs);

        // Load new height tiles and grab the list of what changed.
        let notifications = self.check_downloads_for(MapKind::Heights);

        if !notifications.is_empty() {
            // Apply the changes to each of our core types; flatten first.
            self.check_flatten_heights(&notifications, rt, flattens, selected_tarmac_height)?;
            self.check_foundation_heights(&notifications, rt, foundations);
            self.check_stabilizer_heights(&notifications, stabs);
        }

        // Load color changes; no need to fix up anything.
        self.check_downloads_for(MapKind::Colors);

        Ok(())
    }

    fn check_downloads_for(&mut self, kind: MapKind) -> Vec<MapName> {
        // Note: early return before we are in steady state execution.
        let mut notifications = Vec::new();
        match self.tiff_mut(kind).check_downloads() {
            LoadState::Starting => return notifications,
            LoadState::Preparing(_, _) => return notifications,
            LoadState::Finished => {
                let indices = self.tiff(kind).indices();
                self.lru.add_indices(indices);
                return notifications;
            }
            LoadState::Complete => {
                self.lru.realize_loads(&mut notifications);
            }
        }
        notifications
    }

    fn check_flatten_heights(
        &mut self,
        notifications: &[MapName],
        rt: &RuntimeResource,
        mut flattens: Query<(Entity, &mut Flatten, Option<&Tarmac>)>,
        mut selected_tarmac_height: EventWriter<SelectedTarmacHeight>,
    ) -> Result<()> {
        if !self.is_ready(MapKind::Heights) {
            return Ok(());
        }

        // Seek a target height until we have adequate precision loaded. We want this to
        // happen as early as possible, but not earlier.
        for (ent, mut flatten, tarmac) in flattens.iter_mut() {
            if flatten.target_height().is_none() {
                // Don't allow us to pick _too_ high an overview or our
                // estimate will be way off the actual.
                let min_level = OverviewLevel::from_angular_precision(
                    &(flatten.bounds().east::<Degrees>() - flatten.bounds().west::<Degrees>()).max(
                        flatten.bounds().north::<Degrees>() - flatten.bounds().east::<Degrees>(),
                    ),
                );
                let max_level = OverviewLevel::new(min_level.level() as usize + 1);
                if let Some(grid) =
                    self.get_best_sample_grid(flatten.bounds(), min_level, max_level)
                {
                    let h = grid.average_height();
                    flatten.set_target_height(h);
                    if tarmac.is_some() {
                        selected_tarmac_height.send(SelectedTarmacHeight {
                            tarmac_id: ent,
                            asl: h,
                        });
                    }
                } else {
                    self.request_load_tile_stack(
                        flatten.bounds(),
                        min_level,
                        MapKind::Heights,
                        rt,
                    )?;
                }
            }
        }

        // Flatten any terrain that was loaded before we created the flattening entity.
        //
        // Before caching our foundation intersection data, use the absence as a proxy to know
        // we have not yet seen this entity and should visit our existing terrain data. Note
        // that anything loaded after this point will be intercepted below when we look at
        // notifications.
        //
        // Note: we can't iterate in parallel because of the need to mutate geodb.
        for (_ent, mut flatten, _tarmac) in flattens.iter_mut() {
            // If we don't have a height yet, don't take our pass at the current maps.
            // Note: There is an inherent "race" here. We need maps loaded to know what
            //       average height to flatten to, which implies that there will always
            //       be some maps here to clobber.
            if let Some(height) = flatten.target_height() {
                // This will get cached immediately after we exit this loop, so we'll
                // only have this one opportunity to flatten.
                if !flatten.has_cached_intersections() {
                    // Flatten any flattenable areas in *already loaded* set
                    for level in OverviewLevel::new(0).ascending() {
                        let level = OverviewLevel::new(level as usize);

                        // Subtle note here: we need all of the currently loaded tiles, even
                        // if other tiles in the area are not loaded. Thus, we work by map
                        // rather than grid.
                        let maps = self.tiff(MapKind::Heights)[level]
                            .find_tiles_in_range(flatten.bounds());
                        for map_name in &maps {
                            if self.lru.map_is_ready(map_name) {
                                let grid =
                                    self.get_displaced_grid_in_map(map_name, flatten.bounds());
                                let (lo, hi) = grid.samples_extent_for(map_name);
                                debug_assert_eq!(lo.map_name(), map_name);
                                debug_assert_eq!(hi.map_name(), map_name);
                                self.lru.blit_height_for_grid((lo, hi), height);
                            }
                        }
                    }

                    // Note that we only need the tiff header loaded to know all possible
                    // intersecting maps that might ever be loaded, so there is no race here.
                    flatten.cache_intersections(self);
                }
            }
        }

        // Flatten any maps that were just loaded into the LRU before the terrain
        // system picks them up.
        //
        // Note: we can't iterate in parallel because of the need to mutate geodb.
        for (_ent, flatten, _tarmac) in flattens.iter() {
            if let Some(height) = flatten.target_height() {
                for map_name in notifications {
                    if flatten.intersects_with_map_load(map_name) && self.lru.map_is_ready(map_name)
                    {
                        // Get the portion of the grid that intersects with just this map load.
                        let grid = self.get_displaced_grid_in_map(map_name, flatten.bounds());
                        let (lo, hi) = grid.samples_extent_for(map_name);
                        debug_assert_eq!(lo.map_name(), map_name);
                        debug_assert_eq!(hi.map_name(), map_name);
                        self.lru.blit_height_for_grid((lo, hi), height);
                    }
                }
            }
        }

        Ok(())
    }

    fn check_foundation_heights(
        &mut self,
        notifications: &[MapName],
        rt: &RuntimeResource,
        mut foundations: Query<(&mut Foundation, &mut Frame), Without<GroundStabilizer>>,
    ) {
        // Cache map intersection data on the foundations.
        //
        // Ideally we'd do this when creating the Foundation, but since GeoDb initialization
        // is fully async, we don't know if we'll have the tiff header data needed to compute
        // our intersections until this loop starts running.
        foundations.par_iter_mut().for_each(|(mut foundation, _)| {
            if !foundation.has_cached_intersections() {
                foundation.cache_intersections(self);
            }
        });

        // Update foundations height when intersecting terrain tiles load.
        foundations
            .par_iter_mut()
            .for_each(|(mut foundation, mut frame)| {
                for map_name in notifications {
                    if foundation.improves_refinement(map_name)
                        && foundation.intersects_with_map_load(map_name)
                    {
                        // FIXME: should we adjust the level even if we can't get a full grid?
                        //        I think it would be reasonable to ignore errors here and instead
                        //        clamp to max, but both behaviors are arguably correct.
                        if let Some(grid) = self.get_displaced_full_grid_at_level(
                            foundation.bounds(),
                            1,
                            map_name.level(),
                        ) {
                            // Note: use the lowest height of any corner; it's preferable to
                            // have things clipping into terrain vs floating above it.
                            let new_height = grid.lowest_corner_height(foundation.bounds());
                            frame.set_asl(new_height - foundation.offset_to_ground());
                            foundation.note_refinement(map_name, rt);
                        } else {
                            log::warn!(
                                "skipping foundation leveling because could not get full grid"
                            );
                        }
                    }
                }
            });
    }

    fn capture_current_stabilizer_heights(
        &mut self,
        stabs: &mut Query<
            (&mut GroundStabilizer, &CollisionImpostor, &mut Frame),
            Without<Foundation>,
        >,
    ) {
        stabs.par_iter_mut().for_each(|(mut stab, _, frame)| {
            if let Some(height) = self.get_height_immediate(frame.geodetic().geode(), &meters!(1)) {
                stab.set_current_height(height);
            }
        });
    }

    // This system's job is to make sure that systems that use terrain heights don't need to
    // worry about the terrain height shifting about randomly.
    //
    //  1a) If the prior ground height (stored in the component) is outside of intersection
    //      range, but the new height puts it in range (or below), adjust the frame such that
    //      it has the same offset to the ground as before, regardless of the terrain adjustment
    //      this may result in some weird instantaneous flight model changes (pressure, etc), but
    //      is not going to break anyone's landing or cause someone to fly into terrain that is
    //      busily popping-in under them. (Though in front of them may still be a problem.)
    //  1b) If the prior ground height was in interaction range (stored in the component), then
    //      we need to adjust to follow. (Same as case 1).
    //   2) The ground is below us and adjustments don't interact: keep on trucking.
    //
    //   We're running this system before things needing a height. Movement between prior run and
    //   now doesn't really matter: the other system will have responded to collision or whatever.
    //   Moving the frame around subsequently will not change the situation.
    fn check_stabilizer_heights(
        &mut self,
        notifications: &[MapName],
        mut stabs: Query<
            (&mut GroundStabilizer, &CollisionImpostor, &mut Frame),
            Without<Foundation>,
        >,
    ) {
        stabs
            .par_iter_mut()
            .for_each(|(stab, collider, mut frame)| {
                let corners = collider
                    .shape()
                    .local_aabb()
                    .vertices()
                    .map(|p| frame.local_to_world(p.into()));
                let gabb = GeodeticBB::from_world_space_points(&corners);
                let bounds = GeodeBB::from_geodetic_bb(&gabb);

                if let Some(intersecting) =
                    self.compute_intersecting_maps(MapKind::Heights, &bounds)
                {
                    for notification in notifications {
                        if intersecting.contains(notification) {
                            let old_height = stab.current_height();
                            let new_height = self
                                .get_height_immediate(frame.geodetic().geode(), &meters!(1))
                                .expect("have a prior height here");

                            if new_height != old_height {
                                // If the new height did or would intersect the bounding sphere
                                // (e.g. close enough that we'd notice and be annoyed), update
                                // our position immediately.
                                let sphere = collider.shape().local_bounding_sphere();
                                let mut asl = frame.altitude_asl();
                                let bottom = asl - meters!(sphere.radius());
                                if bottom <= old_height || bottom <= new_height {
                                    asl += new_height - old_height;
                                    frame.set_asl(asl);
                                }
                            }
                        }
                    }
                }
            });
    }

    fn sys_apply_tarmac_height(
        mut tarmacs: Query<(&Tarmac, &mut Frame)>,
        mut buildings: Query<(&TarmacBuilding, &mut Frame), Without<Tarmac>>,
        mut selected_tarmac_height: EventReader<SelectedTarmacHeight>,
    ) -> Result<()> {
        for evt in selected_tarmac_height.read() {
            if let Ok((tarmac, mut frame)) = tarmacs.get_mut(evt.tarmac_id) {
                println!("Got a tarmac height! {evt:?}");
                frame.set_asl(evt.asl - tarmac.offset_to_ground());

                for building_id in tarmac.buildings() {
                    if let Ok((building, mut frame)) = buildings.get_mut(*building_id) {
                        frame.set_asl(evt.asl - building.offset_to_ground());
                    }
                }
            }
        }
        Ok(())
    }

    #[method]
    pub fn memory_cache_size(&self) -> i64 {
        self.lru.memory_cache_size() as i64
    }

    #[method]
    pub fn describe_heights(&self) -> String {
        self.heights.describe()
    }

    #[method]
    pub fn describe_colors(&self) -> String {
        self.colors.describe()
    }

    pub fn is_ready(&self, kind: MapKind) -> bool {
        match kind {
            MapKind::Heights => self.heights.is_ready(),
            MapKind::Colors => self.colors.is_ready(),
        }
    }

    pub fn startup_progress(&self, kind: MapKind) -> f64 {
        match self.tiff(kind).get_state() {
            LoadState::Starting => 0.,
            LoadState::Preparing(current, total) => *current as f64 / *total as f64,
            LoadState::Finished | LoadState::Complete => 1.,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use absolute_unit::degrees;
    use geodesy::Geode;
    use runtime::StdPathsOpts;

    #[test]
    fn it_works() -> Result<()> {
        assert_eq!(mem::size_of::<MapName>(), 8);

        let mut runtime = Runtime::new()?;
        runtime
            .load_extension_with::<StdPaths>(StdPathsOpts::new("nitrogen"))?
            .load_extension_with::<GeoDb>(GeoDbOpts::default())?;
        while !runtime.resource::<GeoDb>().is_ready(MapKind::Heights) {
            runtime.run_sim_once();
            runtime.run_frame_once();
        }
        let a = Geode::new(degrees!(34.415_513_9), degrees!(-119.731_024_1));
        let b = Geode::new(degrees!(34.423_609_5), degrees!(-119.724_560_7));
        for depth in (0..=10).rev() {
            runtime.resource_scope(|heap, mut geodb: Mut<GeoDb>| {
                geodb.map_region_to_tiles(
                    &GeodeBB::from_bounds(a, b),
                    TileSubdivisionLevel::new(depth),
                    MapKind::Heights,
                    heap.resource::<RuntimeResource>(),
                );
            });
        }
        let a = Geode::new(degrees!(34.415_513_9), degrees!(-119.731_024_1));
        let b = Geode::new(degrees!(35.415_513_9), degrees!(-118.731_024_1));
        for depth in (0..=10).rev() {
            runtime.resource_scope(|heap, mut geodb: Mut<GeoDb>| {
                geodb.map_region_to_tiles(
                    &GeodeBB::from_bounds(a, b),
                    TileSubdivisionLevel::new(depth),
                    MapKind::Heights,
                    heap.resource::<RuntimeResource>(),
                );
            });
        }
        Ok(())
    }

    #[test]
    fn test_sampling() -> Result<()> {
        let mut runtime = Runtime::new()?;
        runtime
            .load_extension_with::<StdPaths>(StdPathsOpts::new("nitrogen"))?
            .load_extension_with::<GeoDb>(GeoDbOpts::default())?;
        while !runtime.resource::<GeoDb>().is_ready(MapKind::Heights) {
            runtime.run_sim_once();
            runtime.run_frame_once();
        }

        let everest = Geode::new(degrees!(27.988_056), degrees!(86.925_278));
        let req_region = GeodeBB::from_bounds(
            everest,
            Geode::new(
                everest.lat::<Degrees>() + degrees!(0.1),
                everest.lon::<Degrees>() + degrees!(0.1),
            ),
        );
        let query_region = GeodeBB::from_bounds(
            Geode::new(
                everest.lat::<Degrees>() + degrees!(0.001),
                everest.lon::<Degrees>() + degrees!(0.001),
            ),
            Geode::new(
                everest.lat::<Degrees>() + degrees!(0.002),
                everest.lon::<Degrees>() + degrees!(0.002),
            ),
        );

        let mut samples = None;
        while samples.is_none() {
            samples = runtime
                .resource_mut::<GeoDb>()
                .get_displaced_full_grid_at_level(&query_region, 1, OverviewLevel::new(0));
            for depth in (0..=10).rev() {
                runtime.resource_scope(|heap, mut geodb: Mut<GeoDb>| {
                    geodb.map_region_to_tiles(
                        &req_region,
                        TileSubdivisionLevel::new(depth),
                        MapKind::Heights,
                        heap.resource::<RuntimeResource>(),
                    );
                });
            }
            runtime
                .resource_mut::<GeoDb>()
                .check_downloads_for(MapKind::Heights);
        }
        let height = samples.unwrap().average_height();
        assert!(height > meters!(6000));
        Ok(())
    }

    #[test]
    fn test_height_mesh() -> Result<()> {
        let mut runtime = Runtime::new()?;
        runtime
            .load_extension_with::<StdPaths>(StdPathsOpts::new("nitrogen"))?
            .load_extension_with::<GeoDb>(GeoDbOpts::default())?;
        while !runtime.resource::<GeoDb>().is_ready(MapKind::Heights) {
            runtime.run_sim_once();
            runtime.run_frame_once();
        }

        let everest = Geode::new(degrees!(27.988_056), degrees!(86.925_278));
        let region = GeodeBB::from_bounds(
            everest,
            Geode::new(
                everest.lat::<Degrees>() + degrees!(0.1),
                everest.lon::<Degrees>() + degrees!(0.1),
            ),
        );

        let mut trimesh = None;
        while trimesh.is_none() {
            trimesh = runtime
                .resource_mut::<GeoDb>()
                .get_best_sample_grid(
                    &region,
                    OverviewLevel::from_precision(&meters!(0)),
                    OverviewLevel::from_precision(&meters!(1_000)),
                )
                .map(|v| v.to_mesh());
            for depth in (0..=10).rev() {
                runtime.resource_scope(|heap, mut geodb: Mut<GeoDb>| {
                    geodb.map_region_to_tiles(
                        &region,
                        TileSubdivisionLevel::new(depth),
                        MapKind::Heights,
                        heap.resource::<RuntimeResource>(),
                    );
                });
            }
            runtime
                .resource_mut::<GeoDb>()
                .check_downloads_for(MapKind::Heights);
        }
        let height = trimesh.unwrap().tris()[0][0].v3().magnitude() - meters!(6_356_766);
        assert!(height > meters!(6000));
        Ok(())
    }
}
