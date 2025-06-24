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
use crate::{GeodeBB, Geodetic};
use absolute_unit::{Angle, AngleUnit, Length, LengthUnit, Pt3, meters, radians};
use std::{f64::consts::PI, fmt::Debug, mem};

/// A geodetically aligned bounding box, defined by low and high positions, mapically.
#[derive(Clone, Debug)]
pub struct GeodeticBB {
    south_west: Geodetic,
    north_east: Geodetic,
}

impl Default for GeodeticBB {
    fn default() -> Self {
        Self {
            south_west: Geodetic::new(radians!(0), radians!(0), meters!(0)),
            north_east: Geodetic::new(radians!(0), radians!(0), meters!(0)),
        }
    }
}

impl GeodeticBB {
    // Put a bounding box around a set of world space coordinates
    pub fn from_world_space_points<Unit: LengthUnit, const N: usize>(
        corners: &[Pt3<Unit>; N],
    ) -> Self {
        let mut min_lat = radians!(f64::MAX);
        let mut max_lat = radians!(f64::MIN);
        let mut min_lon = radians!(f64::MAX);
        let mut max_lon = radians!(f64::MIN);
        let mut min_asl = meters!(f64::MAX);
        let mut max_asl = meters!(f64::MIN);

        for corner in corners {
            let geo = Geodetic::from(corner);
            if geo.lat() < min_lat {
                min_lat = geo.lat();
            }
            if geo.lat() > max_lat {
                max_lat = geo.lat();
            }
            if geo.lon() < min_lon {
                min_lon = geo.lon();
            }
            if geo.lon() > max_lon {
                max_lon = geo.lon();
            }
            if geo.asl() < min_asl {
                min_asl = geo.asl();
            }
            if geo.asl() > max_asl {
                max_asl = geo.asl();
            }
        }

        // Let's talk about the longitude singularities.
        //   The simple case ->
        //     50 - 40 => 10, good!
        //    -40 - -50 => 10, good!
        //      5 - -5 => 10, good!
        //   Crossing the rubicon ->
        //     175 - -175 => 350, bad!
        //   If we cross 180, it's shorter to go the other way:
        //     360 - 350 => 10, good!
        //   But does anything break when we flip the left hand side? No!
        //     west = 175; span = 10
        //     east = 175 + 10 = 185
        //     fixed = 185 - 360 = -175, good!
        //
        // But does the same technique work for Latitude? No.
        //   The simple case ->
        //     20 - 10 => 10, good!
        //    -10 - -20 => 10, good!
        //      5 - -5 => 10, good!
        //   At the pole
        //     5 - 5 => 1, bad!
        //   If we cross 180, we... should be covered by longitude, so just clip to +/-90

        if max_lon - min_lon > radians!(PI) {
            mem::swap(&mut min_lon, &mut max_lon);
        }

        Self {
            south_west: Geodetic::new(min_lat, min_lon, min_asl),
            north_east: Geodetic::new(max_lat, max_lon, max_asl),
        }
    }

    pub fn geode_bb(&self) -> GeodeBB {
        GeodeBB::from_geodetic_bb(self)
    }

    pub fn south<Unit: AngleUnit>(&self) -> Angle<Unit> {
        self.south_west.lat()
    }

    pub fn north<Unit: AngleUnit>(&self) -> Angle<Unit> {
        self.north_east.lat()
    }

    pub fn west<Unit: AngleUnit>(&self) -> Angle<Unit> {
        self.south_west.lon()
    }

    pub fn east<Unit: AngleUnit>(&self) -> Angle<Unit> {
        self.north_east.lon()
    }

    pub fn min_asl<Unit: LengthUnit>(&self) -> Length<Unit> {
        self.south_west.asl()
    }

    pub fn max_asl<Unit: LengthUnit>(&self) -> Length<Unit> {
        self.north_east.asl()
    }

    pub fn low(&self) -> &Geodetic {
        &self.south_west
    }

    pub fn high(&self) -> &Geodetic {
        &self.north_east
    }
}

#[cfg(test)]
mod test {
    #[ignore]
    #[test]
    fn test_crossing_the_rubicon() {
        todo!()
    }
}
