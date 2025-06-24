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
use crate::{Geodetic, earth_radius};
use absolute_unit::prelude::*;
// use anyhow::{ensure, Error, Result};
use glam::{DQuat, DVec2, EulerRot};
// use nitrous::{Dict, Value};
use std::fmt;

#[derive(Clone, Copy, Default, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Geode {
    lat: Angle<Radians>,
    lon: Angle<Radians>,
}

impl Geode {
    pub fn new<Unit: AngleUnit>(lat: Angle<Unit>, lon: Angle<Unit>) -> Self {
        Self {
            lat: radians!(lat),
            lon: radians!(lon),
        }
    }

    pub fn lat<Unit: AngleUnit>(&self) -> Angle<Unit> {
        Angle::<Unit>::from(&self.lat)
    }

    pub fn lon<Unit: AngleUnit>(&self) -> Angle<Unit> {
        Angle::<Unit>::from(&self.lon)
    }

    pub fn set_lat<Unit: AngleUnit>(&mut self, lat: Angle<Unit>) {
        self.lat = radians!(lat);
    }

    pub fn set_lon<Unit: AngleUnit>(&mut self, lon: Angle<Unit>) {
        self.lon = radians!(lon);
    }

    pub fn lat_mut(&mut self) -> &mut Angle<Radians> {
        &mut self.lat
    }

    pub fn lon_mut(&mut self) -> &mut Angle<Radians> {
        &mut self.lon
    }

    pub fn latitude_direction_char(&self) -> char {
        if self.lat < radians!(0_f64) { 'S' } else { 'N' }
    }

    pub fn longitude_direction_char(&self) -> char {
        if self.lon < radians!(0_f64) { 'W' } else { 'E' }
    }

    pub fn sea_level(&self) -> Geodetic {
        self.asl(meters!(0))
    }

    pub fn asl<Unit: LengthUnit>(&self, altitude: Length<Unit>) -> Geodetic {
        Geodetic::new_geode(*self, altitude)
    }

    pub fn geocenter<Unit: LengthUnit>(&self, distance: Length<Unit>) -> Geodetic {
        assert!(distance.f64() >= 0.);
        Geodetic::new_geode(*self, distance - earth_radius())
    }

    pub fn lat_lon<Unit: AngleUnit>(&self) -> DVec2 {
        DVec2::new(self.lat::<Unit>().f64(), self.lon::<Unit>().f64())
    }

    // Rotates a vector at +Z (0N0E) such that it will be at the cartesian location of
    // the latitude and longitude of self.
    pub fn rotation(&self) -> DQuat {
        DQuat::from_euler(
            EulerRot::YXZ,
            self.lon::<Radians>().f64(),
            -self.lat::<Radians>().f64(),
            0_f64,
        )
    }

    // From our self to other by f
    pub fn interpolate(&self, other: &Self, f: f64) -> Self {
        Self {
            lat: self.lat + scalar!(f) * (other.lat - self.lat),
            lon: self.lon + scalar!(f) * (other.lon - self.lon),
        }
    }
}

// impl From<Geode> for Dict {
//     fn from(value: Geode) -> Self {
//         let mut out = Dict::default();
//         out.insert("type", Value::from_str("Geode"));
//         out.insert("lat", Value::from_float(value.lat::<Degrees>().f64()));
//         out.insert("lon", Value::from_float(value.lon::<Degrees>().f64()));
//         out
//     }
// }
//
// impl TryFrom<&Dict> for Geode {
//     type Error = Error;
//
//     fn try_from(value: &Dict) -> Result<Self> {
//         let ty = value.expect_str("type", "Geode")?;
//         ensure!(ty == "Geode");
//         let lat = value.expect_float("lat", "Geode")?;
//         let lon = value.expect_float("lon", "Geode")?;
//         Ok(Self::new(degrees!(lat), degrees!(lon)))
//     }
// }

impl fmt::Display for Geode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.latitude_direction_char(), f)?;
        fmt::Display::fmt(&degrees!(self.lat).abs(), f)?;
        fmt::Display::fmt(&self.longitude_direction_char(), f)?;
        fmt::Display::fmt(&degrees!(self.lon).abs(), f)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rotation() {
        let south = Pt3::new(meters!(0), meters!(-1), meters!(0));
        let geo = Geode::new(degrees!(0), degrees!(90));
        dbg!(geo.rotation() * south);
    }
}
