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
    earth_radius,
    Geode
};
use absolute_unit::prelude::*;
// use anyhow::{ensure, Error, Result};
use approx::{AbsDiffEq, RelativeEq};
use glam::DQuat;
// use nitrous::{Dict, Value};
use std::fmt;

/// A latitude, longitude, and height above sea level. Sea level
/// is not a constant, so this is the "mean" sea level.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Geodetic {
    geode: Geode,
    asl: Length<Meters>,
}

impl Geodetic {
    pub fn new<UnitA: AngleUnit, UnitL: LengthUnit>(
        lat: Angle<UnitA>,
        lon: Angle<UnitA>,
        asl: Length<UnitL>,
    ) -> Self {
        Self {
            geode: Geode::new(lat, lon),
            asl: meters!(asl),
        }
    }

    pub fn new_geode<Unit: LengthUnit>(geode: Geode, asl: Length<Unit>) -> Self {
        Self {
            geode,
            asl: meters!(asl),
        }
    }

    pub fn lat<Unit: AngleUnit>(&self) -> Angle<Unit> {
        self.geode.lat::<Unit>()
    }

    pub fn lon<Unit: AngleUnit>(&self) -> Angle<Unit> {
        self.geode.lon::<Unit>()
    }

    pub fn asl<Unit: LengthUnit>(&self) -> Length<Unit> {
        Length::<Unit>::from(&self.asl)
    }

    pub fn geocenter<Unit: LengthUnit>(&self) -> Length<Unit> {
        Length::<Unit>::from(&(self.asl + earth_radius()))
    }

    pub fn set_lat<Unit: AngleUnit>(&mut self, lat: Angle<Unit>) {
        self.geode.set_lat::<Unit>(lat)
    }

    pub fn set_lon<Unit: AngleUnit>(&mut self, lon: Angle<Unit>) {
        self.geode.set_lon::<Unit>(lon)
    }

    pub fn set_asl<Unit: LengthUnit>(&mut self, asl: Length<Unit>) {
        self.asl = meters!(asl);
    }

    pub fn set_geocenter<Unit: LengthUnit>(&mut self, distance: Length<Unit>) {
        self.asl = meters!(distance) - earth_radius();
    }

    pub fn lat_mut(&mut self) -> &mut Angle<Radians> {
        self.geode.lat_mut()
    }

    pub fn lon_mut(&mut self) -> &mut Angle<Radians> {
        self.geode.lon_mut()
    }

    pub fn asl_mut(&mut self) -> &mut Length<Meters> {
        &mut self.asl
    }

    pub fn to_lat_lon_array<T: AngleUnit>(&self) -> [f32; 2] {
        [self.lat::<T>().f32(), self.lon::<T>().f32()]
    }

    pub fn geode(&self) -> &Geode {
        &self.geode
    }

    // Rotates a vector at +Z (0N0E) such that it will be at the cartesian location of
    // the latitude and longitude of self.
    pub fn rotation(&self) -> DQuat {
        self.geode.rotation()
    }

    pub fn pt3<Unit: LengthUnit>(&self) -> Pt3<Unit> {
        let lat = self.geode.lat::<Radians>();
        let lon = self.geode.lon::<Radians>();
        let base = self.asl + earth_radius();
        Pt3::<Unit>::from(&Pt3::<Meters>::new(
            -base * lon.sin() * lat.cos(),
            base * lat.sin(),
            base * lon.cos() * lat.cos(),
        ))
    }

    // From our self to other by f
    pub fn interpolate(&self, other: &Self, f: f64) -> Self {
        Self {
            geode: self.geode.interpolate(&other.geode, f),
            asl: self.asl + scalar!(f) * (other.asl - self.asl),
        }
    }
}

impl<Unit> From<&Pt3<Unit>> for Geodetic
where
    Unit: LengthUnit,
{
    fn from(value: &Pt3<Unit>) -> Self {
        let [x, y, z] = value.to_array();
        let distance = (x * x + y * y + z * z).sqrt();
        let lon = -x.atan2(&z);
        let lat = (y / distance).asin();
        Self::new(lat, lon, meters!(distance) - earth_radius())
    }
}

impl<Unit> From<Pt3<Unit>> for Geodetic
where
    Unit: LengthUnit,
{
    fn from(value: Pt3<Unit>) -> Self {
        Self::from(&value)
    }
}

// impl From<Geodetic> for Dict {
//     fn from(value: Geodetic) -> Self {
//         let mut out = Dict::default();
//         out.insert("type", Value::from_str("Geodetic"));
//         out.insert("lat", Value::from_float(value.lat::<Degrees>().f64()));
//         out.insert("lon", Value::from_float(value.lon::<Degrees>().f64()));
//         out.insert("asl", Value::from_float(value.asl::<Meters>().f64()));
//         out
//     }
// }
//
// impl TryFrom<&Dict> for Geodetic {
//     type Error = Error;
//
//     fn try_from(value: &Dict) -> Result<Self> {
//         let ty = value.expect_str("type", "Geodetic")?;
//         ensure!(ty == "Geodetic");
//         let lat = value.expect_float("lat", "Geodetic")?;
//         let lon = value.expect_float("lon", "Geodetic")?;
//         let asl = value.expect_float("asl", "Geodetic")?;
//         Ok(Self::new(degrees!(lat), degrees!(lon), meters!(asl)))
//     }
// }

impl fmt::Display for Geodetic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.geode, f)?;
        fmt::Display::fmt("@", f)?;
        fmt::Display::fmt(&self.asl, f)
    }
}

impl AbsDiffEq for Geodetic {
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.asl.f64().abs_diff_eq(&other.asl.f64(), epsilon)
            && self
                .geode
                .lat::<Radians>()
                .f64()
                .abs_diff_eq(&other.geode.lat::<Radians>().f64(), epsilon)
            && self
                .geode
                .lon::<Radians>()
                .f64()
                .abs_diff_eq(&other.geode.lon::<Radians>().f64(), epsilon)
    }
}

impl RelativeEq for Geodetic {
    fn default_max_relative() -> Self::Epsilon {
        f64::default_epsilon()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.asl
            .f64()
            .relative_eq(&other.asl.f64(), epsilon, max_relative)
            && self.geode.lat::<Radians>().f64().relative_eq(
                &other.geode.lat::<Radians>().f64(),
                epsilon,
                max_relative,
            )
            && self.geode.lon::<Radians>().f64().relative_eq(
                &other.geode.lon::<Radians>().f64(),
                epsilon,
                max_relative,
            )
    }
}

#[allow(clippy::type_complexity)]
#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;

    const SC45: f64 = std::f64::consts::FRAC_1_SQRT_2;

    static IRREVERSIBLE: [((f64, f64), (f64, f64, f64)); 14] = {
        let r = earth_radius().f64();
        [
            // Full latitude should cancel out most dimensions
            ((-90., 0.), (0., -r, 0.)),
            ((-90., 90.), (0., -r, 0.)),
            ((-90., 180.), (0., -r, 0.)),
            ((-90., -90.), (0., -r, 0.)),
            ((-90., -180.), (0., -r, 0.)),
            ((90., 0.), (0., r, 0.)),
            ((90., 90.), (0., r, 0.)),
            ((90., 180.), (0., r, 0.)),
            ((90., -90.), (0., r, 0.)),
            ((90., -180.), (0., r, 0.)),
            ((0., 180.), (0., 0., -r)),
            ((0., -180.), (0., 0., -r)),
            ((45., 180.), (0., r * SC45, -r * SC45)),
            ((45., -180.), (0., r * SC45, -r * SC45)),
        ]
    };

    static EQUATOR: [((f64, f64), (f64, f64, f64)); 7] = {
        let r = earth_radius().f64();
        [
            // Full sweep at equator to verify we have the right quadrants for everything.
            ((0., 0.), (0., 0., r)),
            ((0., 45.), (-r * SC45, 0., r * SC45)),
            ((0., 90.), (-r, 0., 0.)),
            ((0., 135.), (-r * SC45, 0., -r * SC45)),
            ((0., -135.), (r * SC45, 0., -r * SC45)),
            ((0., -90.), (r, 0., 0.)),
            ((0., -45.), (r * SC45, 0., r * SC45)),
        ]
    };

    static NORTH: [((f64, f64), (f64, f64, f64)); 7] = {
        let r = earth_radius().f64();
        [
            // Full Sweep at +45 latitude
            ((45., 0.), (0., r * SC45, r * SC45)),
            ((45., 45.), (-r * SC45 * SC45, r * SC45, r * SC45 * SC45)),
            ((45., 90.), (-r * SC45, r * SC45, 0.)),
            ((45., 135.), (-r * SC45 * SC45, r * SC45, -r * SC45 * SC45)),
            ((45., -135.), (r * SC45 * SC45, r * SC45, -r * SC45 * SC45)),
            ((45., -90.), (r * SC45, r * SC45, 0.)),
            ((45., -45.), (r * SC45 * SC45, r * SC45, r * SC45 * SC45)),
        ]
    };

    #[test]
    fn test_geodetic_to_cartesian() {
        for ((lat, lon), (x, y, z)) in IRREVERSIBLE.iter() {
            let sph = Geodetic::new(degrees!(*lat), degrees!(*lon), meters!(0));
            let cart = Pt3::<Meters>::new(meters!(*x), meters!(*y), meters!(*z));
            println!("at {lat} {lon}: {} vs {}", sph.pt3::<Meters>(), cart);
            assert_relative_eq!(sph.pt3(), cart, epsilon = 1e-9);
        }
        for ((lat, lon), (x, y, z)) in EQUATOR.iter().chain(&NORTH) {
            let sph = Geodetic::new(degrees!(*lat), degrees!(*lon), meters!(0));
            let cart = Pt3::<Meters>::new(meters!(*x), meters!(*y), meters!(*z));
            println!(
                "at {lat} {lon}: {:0.4} vs {:0.4}",
                Geodetic::from(cart),
                sph
            );
            assert_relative_eq!(sph.pt3(), cart, epsilon = 1e-9);
            assert_relative_eq!(Geodetic::from(cart), sph, epsilon = 1e-9);
        }
    }

    #[test]
    fn test_rotation() {
        for ((lat, lon), (x, y, z)) in IRREVERSIBLE.iter() {
            let sph = Geodetic::new(degrees!(*lat), degrees!(*lon), meters!(0));
            let cart = Pt3::<Meters>::new(meters!(*x), meters!(*y), meters!(*z));
            let z = Pt3::<Meters>::new(meters!(0), meters!(0), earth_radius());
            let result = sph.rotation() * z;
            assert_relative_eq!(result, cart, epsilon = 1e-8);
        }
    }
}
