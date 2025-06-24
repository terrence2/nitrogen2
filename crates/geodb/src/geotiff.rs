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
    MAP_SIZE,
    cache_access::{Memory, TiffAccess},
    geodb::{MapKind, MapName},
    levels::OverviewLevel,
    tiff::{
        BigIFDFooter, BigIFDHeader, BigIFDTag, BigTiffFileHeader, GeoTiffDirHeader, GeoTiffDirKey,
        GeoTiffKeyId, GeoTiffModelPixelScale, GeoTiffTiePoint, IFDTag, OffsetOrInlineData,
        TiffTagId, TiffType,
    },
};
use absolute_unit::prelude::*;
use anyhow::Context;
use bevy::prelude::*;
// use anyhow::{Context, Result, anyhow, ensure};
use approx::relative_eq;
use crossbeam::channel::{self, Receiver, Sender};
use geodesy::{Bearing, Geode, GeodeBB, Geodetic, PitchCline};
use geometry::TriMesh;
use glam::DVec2;
use itertools::Itertools;
use packed_struct::packed_struct;
use parking_lot::{Mutex, RwLock};
// use rapier3d_f64::parry::{
//     na::{DMatrix, Isometry3, UnitQuaternion, Vector3},
//     shape::HeightField,
// };
use rayon::Scope;
use runtime::ensure;
use smallvec::SmallVec;
use std::{
    collections::HashMap,
    fmt, mem,
    ops::Index,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Debug)]
struct GeoTiffOverviewInfo {
    width: u32,
    height: u32,
    tile_stride: u32,
    tile_rows: u32,
    tile_count: u32,
    tile_offsets_offset: OffsetOrInlineData,
    tile_offsets_size: usize,
    tile_offsets_type: TiffType,
    tile_nbytes_offset: OffsetOrInlineData,
    tile_nbytes_size: usize,
    tile_nbytes_type: TiffType,
}

// Tiles are row major, top to bottom, whereas the coordinate space (WGS84)
// is bottom to top. Usage of tiles should take into account that areas are
// top to bottom.
#[packed_struct]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct MapRef {
    stride: u16,
    offset: u32,
}

impl fmt::Display for MapRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}:{:08}", self.stride(), self.offset())
    }
}

impl MapRef {
    fn new(lon: u32, lat: u32, stride: u32) -> Self {
        debug_assert!(stride < 65_536);
        Self {
            stride: stride as u16,
            offset: lat * stride + lon,
        }
    }

    pub fn index(&self) -> usize {
        self.offset as usize
    }

    /// longitude
    pub fn x(&self) -> u32 {
        self.offset % self.stride as u32
    }

    /// latitude
    pub fn y(&self) -> u32 {
        self.offset / self.stride as u32
    }
}

pub(crate) struct SampleInfo {
    geodetic: Geodetic,
    map_name: MapName,
    is_oob: bool,
    uv: (f64, f64),
}

impl SampleInfo {
    fn new(geode: Geode, map_name: MapName, uv: (f64, f64)) -> Self {
        Self {
            geodetic: geode.asl(meters!(0)),
            map_name,
            is_oob: false,
            uv,
        }
    }

    fn oob(geode: Geode, map_name: MapName, uv: (f64, f64)) -> Self {
        Self {
            geodetic: geode.asl(meters!(0)),
            map_name,
            is_oob: true,
            uv,
        }
    }

    pub(crate) fn displace(&mut self, asl: Length<Meters>) {
        self.geodetic.set_asl(asl);
    }

    pub(crate) fn geodetic(&self) -> &Geodetic {
        &self.geodetic
    }

    pub(crate) fn map_name(&self) -> &MapName {
        &self.map_name
    }

    pub(crate) fn is_oob(&self) -> bool {
        self.is_oob
    }

    pub(crate) fn uv(&self) -> &(f64, f64) {
        &self.uv
    }
}

pub struct SampleGrid {
    // A list of samples, sampled from low to high, longitude major (e.g. x/y ordering).
    samples: SmallVec<[SampleInfo; 4]>,

    // The resolution of this sample grid.
    level: OverviewLevel,

    // Number of sampled areas wide, e.g. longitude axis
    // The sample points grid is width + 1 wide.
    width: u32,

    // Number of sampled areas tall, e.g. latitude axis
    // the sample points grid is height + 1 tall.
    height: u32,
}

impl SampleGrid {
    fn new(
        samples: SmallVec<[SampleInfo; 4]>,
        level: OverviewLevel,
        width: u32,
        height: u32,
    ) -> Self {
        assert_eq!(samples.len(), ((width + 1) * (height + 1)) as usize);
        let obj = Self {
            samples,
            level,
            width,
            height,
        };

        // Check that our samples make sense.
        debug_assert!(
            obj.sample(0, 0).geodetic.lon::<Degrees>()
                < obj.sample(width as usize, 0).geodetic.lon::<Degrees>()
        );
        debug_assert!(
            obj.sample(0, 0).geodetic.lat::<Degrees>()
                < obj.sample(0, height as usize).geodetic.lat::<Degrees>()
        );

        obj
    }

    pub(crate) fn sample(&self, x: usize, y: usize) -> &SampleInfo {
        let s = self.width as usize + 1;
        &self.samples[(y * s) + x]
    }

    pub(crate) fn samples(&self) -> &[SampleInfo] {
        &self.samples
    }

    pub(crate) fn samples_mut(&mut self) -> &mut [SampleInfo] {
        &mut self.samples
    }

    pub(crate) fn lowest_corner_height(&self, bounds: &GeodeBB) -> Length<Meters> {
        let heights = bounds.corners().map(|corner| self.get_height_at(&corner));
        *heights.iter().min().unwrap()
    }

    #[allow(unused)]
    pub(crate) fn highest_corner_height(&self, bounds: &GeodeBB) -> Length<Meters> {
        let heights = bounds.corners().map(|corner| self.get_height_at(&corner));
        *heights.iter().max().unwrap()
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn average_height(&self) -> Length<Meters> {
        let mut out = meters!(0);
        for info in &self.samples {
            out += info.geodetic.asl::<Meters>();
        }
        out / scalar!(self.samples.len())
    }

    pub fn maximum_height(&self) -> Length<Meters> {
        self.samples
            .iter()
            .max_by(|a, b| a.geodetic.asl::<Meters>().cmp(&b.geodetic.asl::<Meters>()))
            .expect("non-empty sample grid")
            .geodetic
            .asl()
    }

    pub fn minimum_height(&self) -> Length<Meters> {
        self.samples
            .iter()
            .min_by(|a, b| a.geodetic.asl::<Meters>().cmp(&b.geodetic.asl::<Meters>()))
            .expect("non-empty sample grid")
            .geodetic
            .asl()
    }

    /// Returns a pair of (min, max). Min may equal max.
    pub fn minmax_height(&self) -> (Length<Meters>, Length<Meters>) {
        let (lo, hi) = self
            .samples
            .iter()
            .minmax_by_key(|v| v.geodetic.asl::<Meters>())
            .into_option()
            .expect("non-empty sample grid");
        (lo.geodetic.asl(), hi.geodetic.asl())
    }

    fn linear_height(geode: &Geode, samples: &[&SampleInfo]) -> Length<Meters> {
        debug_assert_eq!(samples.len(), 4);
        debug_assert!(samples[0].geodetic.lat::<Degrees>() < samples[2].geodetic.lat::<Degrees>());

        let x0 = samples[0].geodetic.lon::<Degrees>();
        let x1 = samples[1].geodetic.lon::<Degrees>();
        let fx = (geode.lon::<Degrees>() - x0) / (x1 - x0);
        let y0 = samples[0].geodetic.lat::<Degrees>();
        let y1 = samples[2].geodetic.lat::<Degrees>();
        let fy = (geode.lat::<Degrees>() - y0) / (y1 - y0);

        let f00 = samples[0].geodetic().asl::<Meters>();
        let f10 = samples[1].geodetic().asl::<Meters>();
        let f01 = samples[2].geodetic().asl::<Meters>();
        let f11 = samples[3].geodetic().asl::<Meters>();

        let l = scalar!(1); // I'm going to hell for this, probably.
        f00 * (l - fx) * (l - fy) + f10 * fx * (l - fy) + f01 * (l - fx) * fy + f11 * fx * fy
    }

    pub fn get_height_at(&self, geode: &Geode) -> Length<Meters> {
        let w = self.width as usize;
        let h = self.height as usize;
        let s = w + 1;
        debug_assert!(geode.lon::<Radians>() >= self.samples[0].geodetic.lon::<Radians>());
        debug_assert!(geode.lon::<Radians>() <= self.samples[w].geodetic.lon::<Radians>());
        debug_assert!(geode.lat::<Radians>() >= self.samples[0].geodetic.lat::<Radians>());
        debug_assert!(geode.lat::<Radians>() <= self.samples[h * s].geodetic.lat::<Radians>());

        let lat = geode.lat::<Degrees>();
        let lon = geode.lon::<Degrees>();
        for x in 1..=w {
            if lon <= self.samples[x].geodetic.lon::<Degrees>() {
                for y in 1..=h {
                    if lat <= self.samples[y * s].geodetic.lat::<Degrees>() {
                        let samples = [
                            &self.samples[(y - 1) * s + (x - 1)],
                            &self.samples[(y - 1) * s + x],
                            &self.samples[y * s + (x - 1)],
                            &self.samples[y * s + x],
                        ];
                        return Self::linear_height(geode, &samples);
                    }
                }
            }
        }

        unreachable!("asked for height outside tile")
    }

    pub(crate) fn samples_extent_for(&self, name: &MapName) -> (&SampleInfo, &SampleInfo) {
        // This is simpler than it sounds. If we're scanning top to bottom, left to right.
        // Then the first sample we encounter will be the top left side. Ditto the last
        // we encounter will be the bottom right. No need to min/max over encountered samples.
        let lo = self
            .samples
            .iter()
            .find(|s| &s.map_name == name)
            .expect("map name not in grid");
        let hi = self
            .samples
            .iter()
            .rev()
            .find(|s| &s.map_name == name)
            .expect("map name not in grid too");
        (lo, hi)
    }

    pub fn to_mesh(&self) -> TriMesh<Meters> {
        // Create triangle and edge index lists for use with collision.
        let stride = self.width() + 1;
        let f = |x: u32, y: u32| -> usize { (y * stride + x) as usize };
        let mut tris = Vec::new();
        let mut edges = Vec::new();
        for y in 0..self.height() {
            for x in 0..self.width() {
                tris.push([f(x, y), f(x + 1, y), f(x + 1, y + 1)]);
                tris.push([f(x, y), f(x + 1, y + 1), f(x, y + 1)]);

                edges.push([f(x, y), f(x + 1, y)]);
                edges.push([f(x, y), f(x, y + 1)]);
                edges.push([f(x, y), f(x + 1, y + 1)]);
                if x == self.width() - 1 {
                    edges.push([f(x + 1, y), f(x + 1, y + 1)]);
                }
                if y == self.height() - 1 {
                    edges.push([f(x, y + 1), f(x + 1, y + 1)]);
                }
            }
        }
        let points = self
            .samples()
            .iter()
            .map(|s| s.geodetic().pt3::<Meters>())
            .collect();
        TriMesh::new(points, tris, edges)
    }

    // pub fn to_height_field(&self) -> (HeightField, Isometry3<f64>) {
    //     let floor_asl = self
    //         .samples
    //         .iter()
    //         .min_by(|&a, &b| a.geodetic.asl::<Meters>().cmp(&b.geodetic.asl()))
    //         .expect("a minimum asl")
    //         .geodetic
    //         .asl::<Meters>();
    //
    //     // Origin is in the center of the geode, at sea level, pointing local forward
    //     let lo = *self.sample(0, 0).geodetic().geode();
    //     let hi = *self
    //         .sample(self.width as usize, self.height as usize)
    //         .geodetic()
    //         .geode();
    //     let lat = lo.lat::<Degrees>() + (hi.lat::<Degrees>() - lo.lat::<Degrees>()) / scalar!(2);
    //     let lon = lo.lon::<Degrees>() + (hi.lon::<Degrees>() - lo.lon::<Degrees>()) / scalar!(2);
    //     let center_geo = Geodetic::new(lat, lon, floor_asl);
    //
    //     let px_size = self.level.pixel_extent();
    //
    //     let heights = DMatrix::<f64>::from_row_iterator(
    //         self.height as usize + 1,
    //         self.width as usize + 1,
    //         self.samples
    //             .iter()
    //             .map(|v| (v.geodetic().asl::<Meters>() - floor_asl).f64()),
    //     );
    //     let size_x = px_size * center_geo.lat::<Radians>().cos() * scalar!(self.width);
    //     let size_z = px_size * scalar!(self.height);
    //     let scale = Vector3::new(size_x.f64(), 1.0, size_z.f64());
    //     let field = HeightField::new(heights, scale);
    //
    //     // Points are projected "up" from our basis, so it's important that we pick the tangent
    //     // at the surface, rather than a point at the edge, given the curvature.
    //     let dir = Bearing::north().dvec3_at_geo(PitchCline::level(), center_geo.geode());
    //
    //     let center = center_geo.pt3::<Meters>().na_dvec3();
    //     let up = center.normalize();
    //     let (axis, angle) = UnitQuaternion::face_towards(&dir.into(), &up)
    //         .axis_angle()
    //         .unwrap();
    //     let iframe = Isometry3::new(center, angle * axis.into_inner());
    //     (field, iframe)
    // }
}

#[derive(Debug)]
pub(crate) struct GeoTiffMapIndex {
    kind: MapKind,
    level: OverviewLevel,
    offsets: Memory,
    offsets_type: TiffType,
    nbytes: Memory,
    nbytes_type: TiffType,
}

impl GeoTiffMapIndex {
    #[inline]
    pub(crate) fn at_offset(&self, name: &MapName) -> (u64, usize) {
        debug_assert_eq!(name.kind(), self.kind);
        debug_assert_eq!(name.level(), self.level);

        debug_assert_eq!(self.offsets_type, TiffType::Long8);
        let base = *self
            .offsets
            .as_u64_arr()
            .get(name.position().index())
            .expect("read of offsets in range");

        let size = match self.nbytes_type {
            TiffType::Long8 => {
                let val = *self
                    .nbytes
                    .as_u64_arr()
                    .get(name.position().index())
                    .expect("read of nbytes in range");
                usize::try_from(val).expect("nbytes larger than u32")
            }
            TiffType::Long => {
                let val = *self
                    .nbytes
                    .as_u32_arr()
                    .get(name.position().index())
                    .expect("read of nbytes32 in range");
                val as usize
            }
            _ => {
                panic!("unexpected nbytes type: {:?}", self.nbytes_type)
            }
        };

        (base, size)
    }
}

pub(crate) type GeoTiffIndices =
    HashMap<(MapKind, OverviewLevel), (Arc<RwLock<GeoTiffMapIndex>>, TiffAccess)>;

#[derive(Debug)]
pub struct GeoTiffOverview {
    kind: MapKind,
    level: OverviewLevel,

    // Number of pixels in `span`. This is less or equal to than tile_rows/stride * MAP_SIZE as the
    // number of random sample widths on a random planet is not guaranteed to be a power of two.
    width: u32,
    height: u32,

    // The angular degrees covered by a pixel. Pixels are square, so only one size needed.
    scale: Angle<Degrees>,

    // the positions of the start and end of the covered area.
    // Tie Start is taken straight from the TiepointTag on the GeoTiff.
    // NOTE!: Conventionally this is the _NORTH_ east point, rather than the south east.
    // Tie end is computed from tie_start and the scale, since our maps are linear.
    // tie_bounds refactors this into a standard GeodeBB for Geode queries.
    tie_start: Geode,
    tie_end: Geode,
    bounds: GeodeBB,

    tile_stride: u32,
    tile_rows: u32,
    tile_count: u32,
    index: Arc<RwLock<GeoTiffMapIndex>>,
}

impl GeoTiffOverview {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn tie_start(&self) -> &Geode {
        &self.tie_start
    }

    pub fn tie_end(&self) -> &Geode {
        &self.tie_end
    }

    pub fn bounds(&self) -> &GeodeBB {
        &self.bounds
    }

    pub fn tie_extent_lat(&self) -> Angle<Radians> {
        self.tie_end.lat::<Radians>() - self.tie_start.lat::<Radians>()
    }

    pub fn tie_extent_lon(&self) -> Angle<Radians> {
        self.tie_end.lon::<Radians>() - self.tie_start.lon::<Radians>()
    }

    pub fn scale(&self) -> Angle<Degrees> {
        self.scale
    }

    pub fn tile_stride(&self) -> u32 {
        self.tile_stride
    }

    pub fn tile_rows(&self) -> u32 {
        self.tile_rows
    }

    pub(crate) fn describe(&self) -> String {
        format!(
            "  {}: {}x{} @ {:0.6}",
            self.level.offset(),
            self.width,
            self.height,
            self.scale
        )
    }

    pub(crate) fn find_tiles_in_range(&self, bb: &GeodeBB) -> SmallVec<[MapName; 9]> {
        assert!(bb.low().lat::<Radians>() < bb.high().lat());
        assert!(bb.low().lon::<Radians>() < bb.high().lon());
        let (min, _, _) = self.tile_for(bb.low());
        let (max, _, _) = self.tile_for(bb.high());
        let mut out = SmallVec::new();
        for x in min.x()..=max.x() {
            for y in max.y()..=min.y() {
                let position = MapRef::new(x, y, self.tile_stride);
                debug_assert!(position.offset() < self.tile_count);
                out.push(MapName::new(self.kind, self.level, position));
            }
        }
        out
    }

    // Gets a collection of samples, expanded to fully overlap the bounds, "grid aligned"
    // such that get_height_nearest will return the proper (south-west) sample in all cases.
    pub(crate) fn sample_expanded_grid(&self, bb: &GeodeBB, border: usize) -> SampleGrid {
        // Note: expand bb to full (approximate) pixels; algorithm is:
        // Project bb edges into pixels and floor or ceil as appropriate.
        // Project back to geodetic coordinates.
        let pix_off_w = (bb.west::<Degrees>() - self.tie_start.lon::<Degrees>()) / self.scale;
        let pix_off_e = (bb.east::<Degrees>() - self.tie_start.lon::<Degrees>()) / self.scale;
        let pix_off_s = (self.tie_start.lat::<Degrees>() - bb.south::<Degrees>()) / self.scale;
        let pix_off_n = (self.tie_start.lat::<Degrees>() - bb.north::<Degrees>()) / self.scale;
        let west = degrees!((pix_off_w.trunc() - scalar!(border)) * self.scale)
            + self.tie_start.lon::<Degrees>();
        let east = degrees!((pix_off_e.untrunc() + scalar!(border)) * self.scale)
            + self.tie_start.lon::<Degrees>();
        let south = self.tie_start.lat::<Degrees>()
            - degrees!((pix_off_s.untrunc() + scalar!(border)) * self.scale);
        let north = self.tie_start.lat::<Degrees>()
            - degrees!((pix_off_n.trunc() - scalar!(border)) * self.scale);

        // Compute number of samples in each direction
        // Note: these should be very close, but might be a few epsilon off, so round to integer
        let mut span_x = ((east - west) / self.scale).round().f64() as u32;
        let mut span_y = ((north - south) / self.scale).round().f64() as u32;
        if span_x == 0 || span_y == 0 {
            error!(
                "attempted to sample an empty grid at {} to {}",
                bb.low(),
                bb.high()
            );
            span_x = span_x.max(1);
            span_y = span_y.max(1);
        }
        let bb = GeodeBB::from_bounds(Geode::new(south, west), Geode::new(north, east));

        let halfstep = self.scale / scalar!(2);
        let mut samples: SmallVec<[SampleInfo; 4]> = SmallVec::new();
        for row in 0..=span_y {
            for col in 0..=span_x {
                // Note that we sample offset by half a pixel so that when we truncate to zero
                // in get_*_nearest, we will be grid aligned, but we still need to return
                // the sample location in the desired pixel location, even though the desired
                // location may not map precisely to a sample because of float precision.
                let geode = Geode::new(
                    bb.south::<Degrees>() + scalar!(row) * self.scale,
                    bb.west::<Degrees>() + scalar!(col) * self.scale,
                );
                let sample_geode = Geode::new(
                    geode.lat::<Degrees>() + halfstep,
                    geode.lon::<Degrees>() + halfstep,
                );
                let (mapref, is_oob, uv) = self.tile_for(&sample_geode);
                let name = MapName::new(self.kind, self.level, mapref);
                let sample = if is_oob {
                    SampleInfo::oob(geode, name, uv)
                } else {
                    SampleInfo::new(geode, name, uv)
                };
                samples.push(sample);
            }
        }

        SampleGrid::new(samples, self.level, span_x, span_y)
    }

    // Returns the nearest mapref and if the request was inside the mapref or OOB.
    pub(crate) fn tile_for(&self, grat: &Geode) -> (MapRef, bool, (f64, f64)) {
        // Unify storage with what is in terrain_tile_access.wgsl, except in f64.
        let grat_xy = DVec2::new(grat.lon::<Radians>().f64(), grat.lat::<Radians>().f64());
        let tie_start_xy = DVec2::new(
            self.tie_start.lon::<Radians>().f64(),
            self.tie_start.lat::<Radians>().f64(),
        );
        let tie_extent_xy = DVec2::new(self.tie_extent_lon().f64(), self.tie_extent_lat().f64());

        // Map from the vertex position in graticule radians into the logical position within
        // the index based on the tie points from the COG.
        let tie_to_grat_xy = grat_xy - tie_start_xy;
        let logical_uv_full = tie_to_grat_xy / tie_extent_xy;

        // Wrap around longitude lookups; clamp latitude lookups
        let logical_uv = DVec2::new(logical_uv_full.x % 1., logical_uv_full.y.clamp(0., 1.));

        // If we applied wrapping or clamping, we were out of bounds.
        let oob = logical_uv != logical_uv_full;

        // The index is 1-px per tile, but the actual sizes don't line up with tiles,
        // so we pass in the exact floating extent to map into, combined with expansion
        // into pixel space (such that we can truncate to pixels to get the actual position).
        let scale_tie_to_px = DVec2::new(
            self.width() as f64 / MAP_SIZE as f64,
            self.height() as f64 / MAP_SIZE as f64,
        );

        // It is important that we compute our atlas uv in index space. We scaled
        // by the index size above to get a uv within the pixel. If we were to
        // compute the uv in world/atlas space, we'd be using a different scaling,
        // which might land in a different pixel within the error bars. However,
        // we've already selected the atlas slot, so this kind of error wraps and
        // tells us to select atlas pixels from the wrong side of the tile.
        let index_px_f = logical_uv * scale_tie_to_px;

        // The tile is now the integer part.
        let index_px_i = index_px_f.floor();
        let mapref = MapRef::new(index_px_i.x as u32, index_px_i.y as u32, self.tile_stride);

        // The uv within the tile is the fractional part.
        let uv = index_px_f.fract();

        (mapref, oob, uv.into())
    }
}

// We need to download all tile layer extents before we can return heights. We don't want to
// block loading all the things, however, so load async and make sure we have everything we
// absolutely need before moving into steady-state operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LoadState {
    // Loading geotiff info has not yet begun
    Starting,

    // We are making progress, but are not yet ready for normal operation
    Preparing(usize, usize),

    // State matching startup status "finished". We have to do work before we can move
    // into our runtime state "complete", however, so we have a separate state here.
    Finished,

    // System is up and running normally
    Complete,
}

#[derive(Debug)]
enum StartupStatus {
    Status(usize, usize),
    Failed(String),
    Finished(Vec<GeoTiffOverview>),
}

// Maps a tiff into a simple read interface with tiling, caching, etc.
#[derive(Debug)]
pub struct GeoTiff {
    kind: MapKind,
    location: String,
    reader: TiffAccess,
    state: LoadState,
    layers: Vec<GeoTiffOverview>,
    startup_recv: Receiver<StartupStatus>,
}

impl Index<OverviewLevel> for GeoTiff {
    type Output = GeoTiffOverview;
    fn index(&self, index: OverviewLevel) -> &Self::Output {
        &self.layers[index.offset()]
    }
}

impl GeoTiff {
    pub fn base_layer(&self) -> &GeoTiffOverview {
        &self.layers[0]
    }

    pub fn num_layers(&self) -> usize {
        self.layers.len()
    }

    pub(crate) fn describe(&self) -> String {
        let mut out = String::new();
        out += &format!("{:?}:\n", self.kind);
        out += &format!("  url: '{}'\n", self.location);
        out += &format!("  load state: {:?}\n", self.state);
        for layer in &self.layers {
            out += &layer.describe();
            out += "\n";
        }
        out
    }

    pub(crate) fn new(cache_dir: Option<&Path>, cog_url: &str, kind: MapKind) -> Result<Self> {
        let reader = TiffAccess::new(cache_dir, cog_url)?;
        let (startup_send, startup_recv) = channel::unbounded();
        let tileset = Self {
            kind,
            location: cog_url.to_owned(),
            reader: reader.clone(),
            state: LoadState::Starting,
            layers: Vec::new(),
            startup_recv,
        };

        // FIXME: do something sane if we don't have internet access... flat tile set?

        let cog_url = cog_url.to_owned();
        #[cfg(not(target_arch = "wasm32"))]
        rayon::spawn(move || Self::bg_new(cog_url, kind, reader, startup_send));
        #[cfg(target_arch = "wasm32")]
        Self::bg_new(cog_url, kind, reader, startup_send);

        Ok(tileset)
    }

    pub(crate) fn indices(&self) -> GeoTiffIndices {
        let mut indices = HashMap::new();
        for layer in &self.layers {
            indices.insert(
                (self.kind, layer.level),
                (layer.index.clone(), self.reader.clone()),
            );
        }
        indices
    }

    pub(crate) fn check_downloads(&mut self) -> LoadState {
        if self.state == LoadState::Finished {
            self.state = LoadState::Complete;
        }
        while let Ok(msg) = self.startup_recv.try_recv() {
            match msg {
                StartupStatus::Status(n, d) => {
                    self.state = LoadState::Preparing(n, d);
                }
                StartupStatus::Failed(msg) => {
                    println!("Failed to start tile set server: {}", msg);
                    self.state = LoadState::Finished;
                }
                StartupStatus::Finished(layers) => {
                    self.layers = layers;
                    self.state = LoadState::Finished;
                }
            }
        }
        self.state
    }

    fn bg_new(
        cog_url: String,
        kind: MapKind,
        mut reader: TiffAccess,
        startup_send: Sender<StartupStatus>,
    ) {
        // Download the header chunk and parse out the layer data
        let (layers, scale, tie_point) =
            match Self::bg_parse_cog_header(&cog_url, kind, &mut reader) {
                Ok(layers) => layers,
                Err(err) => {
                    startup_send
                        .send(StartupStatus::Failed(format!("{:?}", err)))
                        .ok();
                    return;
                }
            };

        // Figure out how much work we need to do
        let total = layers
            .iter()
            .map(|layer| layer.tile_offsets_size + layer.tile_nbytes_size)
            .sum::<usize>()
            * 2;
        startup_send.send(StartupStatus::Status(0, total)).ok();

        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        pub enum MMapStartupKind {
            Offsets,
            NumBytes,
        }

        #[derive(Debug)]
        struct MMapStartupResult {
            layer: usize,
            kind: MMapStartupKind,
            map_rv: Result<Memory>,
        }

        // Collect completed maps here
        let maps: Arc<Mutex<Vec<MMapStartupResult>>> = Arc::new(Mutex::new(Vec::new()));

        fn show_progress(
            delta: usize,
            total: usize,
            progress: &Arc<AtomicUsize>,
            sender: &Sender<StartupStatus>,
        ) {
            progress.fetch_add(delta, Ordering::SeqCst);
            sender
                .send(StartupStatus::Status(
                    progress.load(Ordering::SeqCst),
                    total,
                ))
                .ok();
        }

        fn map_cached(
            scope: &Scope,
            (layer, kind, maps): (usize, MMapStartupKind, Arc<Mutex<Vec<MMapStartupResult>>>),
            (total, progress, sender): (usize, Arc<AtomicUsize>, Sender<StartupStatus>),
            (reader, offset, size): (TiffAccess, OffsetOrInlineData, usize),
        ) {
            scope.spawn(move |_| {
                let map_rv = match offset {
                    OffsetOrInlineData::Offset(offset) => reader.mmap_cache(
                        offset,
                        size,
                        |delta: usize| {
                            show_progress(delta, total, &progress, &sender);
                        },
                        |delta: usize| {
                            show_progress(delta, total, &progress, &sender);
                        },
                    ),
                    OffsetOrInlineData::Data4(data) => Ok(Memory::Inline4(data)),
                    OffsetOrInlineData::Data8(data) => Ok(Memory::Inline8(data)),
                };
                maps.lock().push(MMapStartupResult {
                    layer,
                    kind,
                    map_rv,
                });
            });
        }

        // Download all tile offset info, using as much parallelism as possible
        let progress_bytes = Arc::new(AtomicUsize::new(0));
        rayon::scope(|s| {
            for (i, layer) in layers.iter().enumerate() {
                map_cached(
                    s,
                    (i, MMapStartupKind::Offsets, maps.clone()),
                    (total, progress_bytes.clone(), startup_send.clone()),
                    (
                        reader.clone(),
                        layer.tile_offsets_offset,
                        layer.tile_offsets_size,
                    ),
                );
                map_cached(
                    s,
                    (i, MMapStartupKind::NumBytes, maps.clone()),
                    (total, progress_bytes.clone(), startup_send.clone()),
                    (
                        reader.clone(),
                        layer.tile_nbytes_offset,
                        layer.tile_nbytes_size,
                    ),
                );
            }
        });

        assert_eq!(maps.lock().len(), layers.len() * 2);
        let mut s = scale;
        let mut out = Vec::new();
        for (mut chunk, layer) in maps
            .lock()
            .drain(..)
            .sorted_by_key(|rv| (rv.layer, rv.kind))
            .chunks(2)
            .into_iter()
            .zip(layers)
        {
            let offsets_rv = chunk.next().expect("offsets");
            let level = OverviewLevel::new(offsets_rv.layer);
            let nbytes_rv = chunk.next().expect("nbytes");
            trace!(
                "Layer: {kind:?} @ {:?} => {:#?}",
                OverviewLevel::new(offsets_rv.layer),
                layer
            );
            assert_eq!(offsets_rv.kind, MMapStartupKind::Offsets);
            assert_eq!(nbytes_rv.kind, MMapStartupKind::NumBytes);
            assert_eq!(offsets_rv.layer, nbytes_rv.layer);
            let offsets_map = match offsets_rv.map_rv {
                Ok(map) => map,
                Err(err) => {
                    println!("failed to load offsets for layer {}", offsets_rv.layer);
                    startup_send
                        .send(StartupStatus::Failed(format!(
                            "failed to load offsets for layer {}: {err:?}",
                            offsets_rv.layer
                        )))
                        .ok();
                    return;
                }
            };
            let nbytes_map = match nbytes_rv.map_rv {
                Ok(map) => map,
                Err(err) => {
                    println!("failed to load nbytes for layer {}", nbytes_rv.layer);
                    startup_send
                        .send(StartupStatus::Failed(format!(
                            "failed to load nbytes for layer {}: {err:?}",
                            nbytes_rv.layer
                        )))
                        .ok();
                    return;
                }
            };
            let tie_end = Geode::new(
                tie_point.lat::<Degrees>() - s * scalar!(layer.height),
                tie_point.lon::<Degrees>() + s * scalar!(layer.width),
            );
            let overview = GeoTiffOverview {
                kind,
                level,
                width: layer.width,
                height: layer.height,
                scale: s,
                tie_start: tie_point,
                tie_end,
                bounds: GeodeBB::from_bounds(
                    Geode::new(tie_end.lat::<Radians>(), tie_point.lon::<Radians>()),
                    Geode::new(tie_point.lat::<Radians>(), tie_end.lon::<Radians>()),
                ),
                tile_stride: layer.tile_stride,
                tile_rows: layer.tile_rows,
                tile_count: layer.tile_count,
                index: Arc::new(RwLock::new(GeoTiffMapIndex {
                    kind,
                    level,
                    offsets: offsets_map,
                    offsets_type: layer.tile_offsets_type,
                    nbytes: nbytes_map,
                    nbytes_type: layer.tile_nbytes_type,
                })),
            };
            out.push(overview);
            s *= scalar!(2);
        }

        startup_send.send(StartupStatus::Finished(out)).ok();
    }

    pub(crate) fn get_state(&self) -> &LoadState {
        &self.state
    }

    pub(crate) fn is_ready(&self) -> bool {
        matches!(self.state, LoadState::Finished | LoadState::Complete)
    }

    fn bg_parse_cog_header(
        cog_url: &str,
        kind: MapKind,
        reader: &mut TiffAccess,
    ) -> Result<(Vec<GeoTiffOverviewInfo>, Angle<Degrees>, Geode)> {
        let header_data = reader
            .get(0, 8192, |_| {}, |_| {})
            .map_err(|err| anyhow::anyhow!(err.to_string()))
            .with_context(|| format!("getting header for {cog_url}"))?;
        let header = BigTiffFileHeader::overlay(&header_data[0..BigTiffFileHeader::size_of()])?;
        header.validate_support()?;

        // Parse the header data we just downloaded in order to find out how
        // big the tile offsets and sizes are that we need to download and map
        // before we can enter a ready state.
        let mut layers = Vec::new();
        let mut ifd_offset = usize::try_from(header.offset_to_first_ifd())?;
        let (mut next_ifd, scale, tie_point) =
            Self::bg_parse_main_ifd(ifd_offset, &header_data, kind, &mut layers)?;
        while next_ifd != 0 {
            ifd_offset = usize::try_from(next_ifd)?;
            next_ifd = Self::bg_parse_overview_ifd(ifd_offset, &header_data, kind, &mut layers)?;
        }

        Ok((layers, scale, tie_point))
    }

    // Shared between both main and overview ifds
    fn bg_parse_ifd_common(
        mut offset: usize,
        data: &[u8],
        kind: MapKind,
        layers: &mut Vec<GeoTiffOverviewInfo>,
    ) -> Result<(u64, Vec<IFDTag>)> {
        let ifd_header = BigIFDHeader::overlay(&data[offset..offset + BigIFDHeader::size_of()])?;
        let ntags = usize::try_from(ifd_header.tag_count())?;
        offset += BigIFDHeader::size_of();
        let tags = BigIFDTag::overlay_slice(&data[offset..offset + BigIFDTag::size_of() * ntags])?
            .iter()
            .map(|v| IFDTag::from_big(v).expect("ifd types are sane"))
            .collect::<Vec<_>>();
        offset += BigIFDTag::size_of() * ntags;
        let footer = BigIFDFooter::overlay(&data[offset..offset + BigIFDFooter::size_of()])?;

        // Pull out and assert present all required tags
        let width_tag = IFDTag::find(TiffTagId::ImageWidth, &tags)
            .ok_or_else(|| BevyError::from("cog width tag missing"))?;
        let height_tag = IFDTag::find(TiffTagId::ImageLength, &tags)
            .ok_or_else(|| BevyError::from("cog height tag missing"))?;
        let bps_tag = IFDTag::find(TiffTagId::BitsPerSample, &tags)
            .ok_or_else(|| BevyError::from("cog bits per sample tag missing"))?;
        let compression_tag = IFDTag::find(TiffTagId::Compression, &tags)
            .ok_or_else(|| BevyError::from("cog compression tag missing"))?;
        let photometrics_tag = IFDTag::find(TiffTagId::PhotometricInterpretation, &tags)
            .ok_or_else(|| BevyError::from("cog photometrics tag missing"))?;
        let pixsize_tag = IFDTag::find(TiffTagId::SamplesPerPixel, &tags)
            .ok_or_else(|| BevyError::from("cog photometrics tag missing"))?;
        let sampfmt_tag = IFDTag::find(TiffTagId::SampleFormat, &tags)
            .ok_or_else(|| BevyError::from("cog sample format tag missing"))?;
        let planar_tag = IFDTag::find(TiffTagId::PlanarConfiguration, &tags)
            .ok_or_else(|| BevyError::from("cog planar config tag missing"))?;
        let tile_width_tag = IFDTag::find(TiffTagId::TileWidth, &tags)
            .ok_or_else(|| BevyError::from("cog tile width tag missing"))?;
        let tile_height_tag = IFDTag::find(TiffTagId::TileLength, &tags)
            .ok_or_else(|| BevyError::from("cog tile height tag missing"))?;
        let tile_offsets_tag = IFDTag::find(TiffTagId::TileOffsets, &tags)
            .ok_or_else(|| BevyError::from("cog tile offsets tag missing"))?;
        let tile_nbytes_tag = IFDTag::find(TiffTagId::TileByteCounts, &tags)
            .ok_or_else(|| BevyError::from("cog tile byte counts tag missing"))?;

        // Make assertions about things that should always be true
        ensure!(
            compression_tag.inline_data == 34925,
            "expected lzma compression"
        );
        ensure!(planar_tag.inline_data == 1, "expected chunked planes");
        ensure!(
            tile_width_tag.inline_data == MAP_SIZE as u64,
            "expected 512 pixel tiles"
        );
        ensure!(
            tile_height_tag.inline_data == MAP_SIZE as u64,
            "expected 512 pixel tiles"
        );
        ensure!(
            tile_offsets_tag.field_type == TiffType::Long8,
            "expected u64 offsets"
        );

        // Assert things we expect about the format
        match kind {
            MapKind::Heights => {
                ensure!(bps_tag.inline_data == 16, "expected 16 bit integer heights");
                ensure!(pixsize_tag.inline_data == 1, "expected 1 sample per pix");
                ensure!(sampfmt_tag.inline_data == 2, "expected signed samples");
                ensure!(
                    photometrics_tag.inline_data == 1,
                    "expected 0 is black photometrics"
                );
            }
            MapKind::Colors => {
                ensure!(bps_tag.inline_data == 8, "expected 8 bit integer colors");
                ensure!(pixsize_tag.inline_data == 3, "expected 3 samples per pix");
                ensure!(sampfmt_tag.inline_data == 1, "expected unsigned samples");
                ensure!(
                    photometrics_tag.inline_data == 2,
                    "expected rgb photometrics"
                );
            }
        }

        let tile_offsets_offset = tile_offsets_tag.data_pointer()?;
        let tile_offsets_size = tile_offsets_tag.data_size()?;

        let tile_nbytes_offset = tile_nbytes_tag.data_pointer()?;
        let tile_nbytes_size = tile_nbytes_tag.data_size()?;

        let tile_count0 = tile_offsets_size / tile_offsets_tag.field_type.size_of();
        let tile_count1 = tile_nbytes_size / tile_nbytes_tag.field_type.size_of();
        ensure!(
            tile_count0 == tile_count1,
            "tile offsets and nbytes segment lengths disagree"
        );
        let width = u32::try_from(width_tag.inline_data)?;
        let height = u32::try_from(height_tag.inline_data)?;
        let tile_stride = (width as f64 / 512.).ceil() as u32;
        let tile_rows = (height as f64 / 512.).ceil() as u32;
        let tile_count2 = tile_rows as usize * tile_stride as usize;
        ensure!(
            tile_count0 == tile_count2,
            "tile offsets and nbytes segment lengths disagree with overview size"
        );

        let layer = GeoTiffOverviewInfo {
            width,
            height,
            tile_stride,
            tile_rows,
            tile_count: tile_count0 as u32,
            tile_offsets_offset,
            tile_offsets_size,
            tile_offsets_type: tile_offsets_tag.field_type,
            tile_nbytes_offset,
            tile_nbytes_size,
            tile_nbytes_type: tile_nbytes_tag.field_type,
        };
        layers.push(layer);
        Ok((footer.next_ifd(), tags))
    }

    fn bg_parse_main_ifd(
        offset: usize,
        data: &[u8],
        kind: MapKind,
        layers: &mut Vec<GeoTiffOverviewInfo>,
    ) -> Result<(u64, Angle<Degrees>, Geode)> {
        let (next_ifd, tags) = Self::bg_parse_ifd_common(offset, data, kind, layers)?;

        // Only defined on the top layer
        let scale_tag = IFDTag::find(TiffTagId::ModelPixelScale, &tags)
            .ok_or_else(|| BevyError::from("cog model pixel scale tag missing"))?;
        let tie_point_tag = IFDTag::find(TiffTagId::ModelTiePoint, &tags)
            .ok_or_else(|| BevyError::from("cog model tie point tag missing"))?;
        let geo_dir_tag = IFDTag::find(TiffTagId::GeoKeyDirectoryTag, &tags)
            .ok_or_else(|| BevyError::from("cog geo key director tag missing"))?;

        let scale_offset = usize::try_from(scale_tag.inline_data)?;
        let scale = GeoTiffModelPixelScale::overlay(
            &data[scale_offset..scale_offset + GeoTiffModelPixelScale::size_of()],
        )?;
        ensure!(scale.z() == 0.0, "unexpected z scale");
        ensure!(scale.x() == scale.y(), "expected uniform scaling");
        let scale = degrees!(scale.x());

        let tie_point_offset = usize::try_from(tie_point_tag.inline_data)?;
        let tie_point = GeoTiffTiePoint::overlay(
            &data[tie_point_offset..tie_point_offset + GeoTiffTiePoint::size_of()],
        )?;
        ensure!(tie_point.i() == 0., "tie point i must be 0");
        ensure!(tie_point.j() == 0., "tie point j must be 0");
        ensure!(tie_point.k() == 0., "tie point k must be 0");
        ensure!(tie_point.z() == 0., "tie point z must be 0");
        ensure!(
            relative_eq!(tie_point.x(), -180., epsilon = 1e-12)
                || tie_point.x() == -180. - scale.f64() / 2.,
            "tie point x must be -180-dx"
        );
        ensure!(
            relative_eq!(tie_point.y(), 90., epsilon = 1e-12)
                || tie_point.y() == 60. + scale.f64() / 2.,
            "tie point y must be 60+dx"
        );
        let tie_pt = Geode::new(degrees!(tie_point.y()), degrees!(tie_point.x()));

        let geo_dir_offset = usize::try_from(geo_dir_tag.inline_data)?;
        let geo_dir = GeoTiffDirHeader::overlay(
            &data[geo_dir_offset..geo_dir_offset + GeoTiffDirHeader::size_of()],
        )?;
        ensure!(geo_dir.version() == 1, "cog expected geo dir version 1");
        ensure!(geo_dir.key_revision() == 1, "cog expected geo dir rev 1");
        ensure!(geo_dir.minor_revision() == 1, "cog expected geo dir rev 1");
        let key_count = usize::from(geo_dir.number_of_keys());
        ensure!(
            GeoTiffDirHeader::size_of() + key_count * GeoTiffDirKey::size_of()
                == usize::try_from(geo_dir_tag.count)? * mem::size_of::<u16>(),
            "mismatched geo dir sizes"
        );
        let key_offset = geo_dir_offset + GeoTiffDirHeader::size_of();
        let geo_keys = GeoTiffDirKey::overlay_slice(
            &data[key_offset..key_offset + GeoTiffDirKey::size_of() * key_count],
        )?;
        for key in geo_keys {
            ensure!(key.tiff_tag_location() == 0, "cog expected inline geo tag");
            ensure!(key.count() == 1, "cog expected inline geo tag");
            match GeoTiffKeyId::try_from(key.id())? {
                GeoTiffKeyId::GtModel => {
                    ensure!(key.value_offset() == 2, "expected lat/lon projection")
                }
                GeoTiffKeyId::GtRaster => {
                    ensure!(key.value_offset() == 1, "expected pixel-is-area")
                }
                GeoTiffKeyId::Geographic => {
                    ensure!(key.value_offset() == 4326, "expected GCS_WGS_84 coords")
                }
            }
        }

        Ok((next_ifd, scale, tie_pt))
    }

    fn bg_parse_overview_ifd(
        offset: usize,
        data: &[u8],
        kind: MapKind,
        layers: &mut Vec<GeoTiffOverviewInfo>,
    ) -> Result<u64> {
        let (next_ifd, tags) = Self::bg_parse_ifd_common(offset, data, kind, layers)?;

        let subfile_type_tag = IFDTag::find(TiffTagId::NewSubfileType, &tags)
            .ok_or_else(|| BevyError::from("cog subfile type tag missing"))?;
        ensure!(subfile_type_tag.inline_data == 1, "expected subfile type 1");

        Ok(next_ifd)
    }
}
