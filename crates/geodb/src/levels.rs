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

// A quick explanation of why this algorithm is so dumb looking:
// ---
// As we go higher in latitude, the pixels we need to cover an certain extent of longitude
// shrinks proportional to cos(lat). When the minimum pixel density gets above the nyquist
// limit (2x), we should shrink to using the next size texture map up to save space.
// This crossover point is where cos(x) = 0.5, e.g. 60 degrees latitude. However, srtm and
// most other equatorial height maps stop at 60 degrees latitude. Probably for exactly this
// reason. Thus, we can use the terrain resolution we would use at the equator for all tile
// resolution selection operations, despite the shrinking extent of a degree with latitude.

use crate::MapKind;
use absolute_unit::prelude::*;
use std::ops::RangeInclusive;

// What this table represents:
// ---
// Nitrogen's height map sets the initial level (level 1) as a 90° (both lat and lon) extent.
// At this extent we want 512 px, giving an angular resolution of 90/512. At each subdivision,
// we double the required angular resolution. The SRTM COG works in the opposite direction:
// level 0 is the full extent image and deeper levels are "overviews" that half the resolution.
// In order to find the optimal resolution to load at a particular tile tree depth, we need to
// find the overview level that provides a few more pixels than we require at that depth for
// 90/512. We can easily pre-compute this, e.g. here for SRTM:
//
// tile                                                  pixmap
// depth | degrees    arcsecs    meters     m/pix      | level m/pix    m/tile   "/tile  deg/tile
// ------+---------------------------------------------+------------------------------------------
// 1     | 90         324000     9720000    18984.3    | 9     15360    7864320  262144  72.817
// 2     | 45         162000     4860000    9492.18    | 8     7680     3932160  131072  36.408
// 3     | 22.5       81000      2430000    4746.09    | 7     3840     1966080  65536   18.204
// 4     | 11.25      40500      1215000    2373.04    | 6     1920     983040   32768   9.1022
// 5     | 5.625      20250      607500     1186.52    | 5     960      491520   16384   4.5511
// 6     | 2.8125     10125      303750     593.261    | 4     480      245760   8192    2.2755
// 7     | 1.40625    5062.5     151875     296.630    | 3     240      122880   4096    1.1377
// 8     | 0.703125   2531.25    75937.5    148.315    | 2     120      61440    2048    0.5688
// 9     | 0.351562   1265.62    37968.75   74.1577    | 1     60       30720    1024    0.2844
// 10    | 0.175781   632.812    18984.375  37.0788    | 0     30       15360    512     0.1422
// 11    | 0.087890   316.406    9492.1875  18.5394
// 12    | 0.043945   158.203    4746.0937  9.26971
// 13    | 0.021972   79.1015    2373.0468  4.63485
// 14    | 0.010986   39.5507    1186.5234  2.31742
// 15    | 0.005493   19.7753    593.26171  1.15871
// 16    | 0.002746   9.88769    296.63085  0.57935
// 17    | 0.001373   4.94384    148.31542  0.28967
// 18    | 0.000686   2.47192    74.157714  0.14483
// 19    | 0.000343   1.23596    37.078857  0.07241
// 20    | 0.000171   0.61798    18.539428  0.03620
// 21    | 0.000085   0.30899    9.2697143  0.01810
// 22    | 0.000042   0.15449    4.6348571  0.00905
//
// So, for SRTM, at an input tile depth of x, we need to populate the tile cache with tiles in
// the given extent at overlay layer `(10 - x).clamp(0, 9)`.
#[inline]
fn tile_depth_to_srtm_overview_level(depth: usize) -> u8 {
    (10 - depth).clamp(0, 10) as u8
}

// Similarly for the BMNG data set.
// tile                                                  pixmap
// depth | degrees    arcsecs    meters     m/pix      | level m/pix    m/tile   "/tile  deg/tile
// ------+---------------------------------------------+------------------------------------------
// -2    |                                             | 8 118548.27  60696712.54  2023223.7 562.0
// -1    |                                             | 7  59274.13  30348356.27  1011611.9 281.0
// 0     |                                             | 6  29637.07  15174178.13   505805.9 140.5
// 1     | 90         324000     9720000    18984.3    | 5  14818.53   7587089.07   252903.0  70.2
// 2     | 45         162000     4860000    9492.18    | 4   7409.27   3793544.53   126451.5  35.1
// 3     | 22.5       81000      2430000    4746.09    | 3   3704.63   1896772.27    63225.7  17.6
// 4     | 11.25      40500      1215000    2373.04    | 2   1852.32    948386.13    31612.9   8.8
// 5     | 5.625      20250      607500     1186.52    | 1    926.16    474193.07    15806.4   4.4
// 6     | 2.8125     10125      303750     593.261    | 0    463.08    237096.53     7903.2   2.2
// 7     | 1.40625    5062.5     151875     296.630    |
// 8     | 0.703125   2531.25    75937.5    148.315    |
// 9     | 0.351562   1265.62    37968.75   74.1577    |
// 10    | 0.175781   632.812    18984.375  37.0788    |
// 11    | 0.087890   316.406    9492.1875  18.5394
// 12    | 0.043945   158.203    4746.0937  9.26971
// 13    | 0.021972   79.1015    2373.0468  4.63485
// 14    | 0.010986   39.5507    1186.5234  2.31742
// 15    | 0.005493   19.7753    593.26171  1.15871
// 16    | 0.002746   9.88769    296.63085  0.57935
// 17    | 0.001373   4.94384    148.31542  0.28967
// 18    | 0.000686   2.47192    74.157714  0.14483
// 19    | 0.000343   1.23596    37.078857  0.07241
// 20    | 0.000171   0.61798    18.539428  0.03620
// 21    | 0.000085   0.30899    9.2697143  0.01810
// 22    | 0.000042   0.15449    4.6348571  0.00905
#[inline]
fn tile_depth_to_bmng_overview_level(depth: usize) -> u8 {
    6usize.saturating_sub(depth) as u8
}

// Overview Level; full resolution at level 0, halving resolution and doubling pixel angular
// extent at each subsequent level.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct OverviewLevel(u8);

impl OverviewLevel {
    #[inline]
    pub fn level(&self) -> u8 {
        self.0
    }

    #[inline]
    pub(crate) fn offset(&self) -> usize {
        self.0 as usize
    }

    // From the layout of the file when loading, per cog spec
    #[inline]
    pub(crate) fn new(level: usize) -> Self {
        debug_assert!(level < 256);
        Self(level as u8)
    }

    /// Expand by 2x for each overview level above 0.
    /// This is useful for computing the pixel coverage of an index at a level.
    #[inline]
    pub fn extent_factor(&self) -> f32 {
        (1usize << (self.0 as usize)) as f32
    }

    // Given a desired precision, find the best overview level to query from.
    pub fn from_precision(precision: &Length<Meters>) -> Self {
        if precision < &meters!(30) {
            return Self(0);
        } else if precision < &meters!(60) {
            return Self(1);
        } else if precision < &meters!(120) {
            return Self(2);
        } else if precision < &meters!(240) {
            return Self(3);
        } else if precision < &meters!(480) {
            return Self(4);
        } else if precision < &meters!(960) {
            return Self(5);
        } else if precision < &meters!(1920) {
            return Self(6);
        } else if precision < &meters!(3840) {
            return Self(7);
        } else if precision < &meters!(7680) {
            return Self(8);
        } else if precision < &meters!(15360) {
            return Self(9);
        }
        Self(9)
    }

    pub fn from_angular_precision(precision: &Angle<Degrees>) -> Self {
        let mut arcsec = degrees!(arcseconds!(1));
        for i in 1..=12 {
            if precision < &arcsec {
                return Self(i - 1);
            }
            arcsec *= scalar!(2);
        }
        Self(12)
    }

    // Return the width of a pixel at the current level
    pub fn pixel_extent(&self) -> Length<Meters> {
        scalar!(self.extent_factor()) * meters!(30)
    }

    pub fn pixel_angle(&self) -> Angle<ArcSeconds> {
        scalar!(self.extent_factor()) * arcseconds!(1.)
    }

    // Iterate overview levels from finer resolution to coarser resolution
    pub(crate) fn ascending(&self) -> RangeInclusive<u8> {
        self.0..=12
    }

    pub(crate) fn up_to(&self, other: &Self) -> RangeInclusive<u8> {
        self.0..=other.0
    }

    pub(crate) fn is_more_refined_than(&self, other: Self) -> bool {
        self.0 < other.0
    }
}

/// External interface to map to overviews
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TileSubdivisionLevel(usize);

impl TileSubdivisionLevel {
    #[inline]
    pub fn new(depth: usize) -> Self {
        Self(depth)
    }

    pub fn best_overview_level(&self, kind: MapKind) -> OverviewLevel {
        match kind {
            MapKind::Heights => OverviewLevel(tile_depth_to_srtm_overview_level(self.0)),
            MapKind::Colors => OverviewLevel(tile_depth_to_bmng_overview_level(self.0)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_scale() {
        assert_eq!(
            TileSubdivisionLevel::new(10).best_overview_level(MapKind::Heights),
            OverviewLevel(0)
        );
    }
}
