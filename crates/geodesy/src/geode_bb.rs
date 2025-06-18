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
use crate::{Geode, GeodeticBB};
use absolute_unit::prelude::*;
use std::fmt::Debug;

/// As for GeodeticBB, but without a height component
#[derive(Clone, Debug)]
pub struct GeodeBB {
    south_west: Geode,
    north_east: Geode,
}

impl Default for GeodeBB {
    fn default() -> Self {
        Self {
            south_west: Geode::new(radians!(0), radians!(0)),
            north_east: Geode::new(radians!(0), radians!(0)),
        }
    }
}

impl GeodeBB {
    pub fn from_bounds(south_west: Geode, north_east: Geode) -> Self {
        Self {
            south_west,
            north_east,
        }
    }

    pub fn from_geodetic_bb(bb: &GeodeticBB) -> Self {
        Self {
            south_west: Geode::new(bb.south::<Radians>(), bb.west::<Radians>()),
            north_east: Geode::new(bb.north::<Radians>(), bb.east::<Radians>()),
        }
    }

    pub fn contains(&self, pos: &Geode) -> bool {
        let lat = pos.lat::<Radians>();
        let lon = pos.lon::<Radians>();
        let s = self.south::<Radians>();
        let n = self.north::<Radians>();
        let w = self.west::<Radians>();
        let e = self.east::<Radians>();
        lat >= s && lat <= n && lon >= w && lon <= e
    }

    pub fn expand_with_border<Unit: LengthUnit>(&mut self, dist: Length<Unit>) {
        let dist = Length::<Meters>::from(&dist);
        let delta_lat = degrees!(dist.f64() / 111_111.);
        let delta_lon = self.south_west.lat::<Radians>().cos() * delta_lat;
        *self.south_west.lat_mut() -= delta_lat;
        *self.north_east.lat_mut() += delta_lat;
        *self.south_west.lon_mut() -= delta_lon;
        *self.north_east.lon_mut() += delta_lon;
    }

    pub fn expand_to_include(&mut self, other: &Self) {
        let south = self.south::<Radians>().min(other.south::<Radians>());
        let west = self.west::<Radians>().min(other.west::<Radians>());
        let north = self.north::<Radians>().max(other.north::<Radians>());
        let east = self.east::<Radians>().max(other.east::<Radians>());
        self.south_west = Geode::new(south, west);
        self.north_east = Geode::new(north, east);
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

    pub fn low(&self) -> &Geode {
        &self.south_west
    }

    pub fn high(&self) -> &Geode {
        &self.north_east
    }

    pub fn corners(&self) -> [Geode; 4] {
        [
            self.south_west,
            Geode::new(self.north::<Radians>(), self.west::<Radians>()),
            self.north_east,
            Geode::new(self.south::<Radians>(), self.east::<Radians>()),
        ]
    }

    pub fn center(&self) -> Geode {
        let n = self.north::<Radians>();
        let s = self.south::<Radians>();
        let e = self.east::<Radians>();
        let w = self.west::<Radians>();
        let lat = s + (n - s) / scalar!(2);
        let lon = w + (e - w) / scalar!(2);
        Geode::new(lat, lon)
    }
}
