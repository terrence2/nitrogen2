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
use crate::{Geode, PitchCline};
use absolute_unit::prelude::*;
// use anyhow::{ensure, Error, Result};
use glam::{DQuat, DVec2, DVec3, EulerRot};
// use nitrous::{Dict, Value};
use std::{f64::consts::PI, fmt, ops::Add};

/// Represents a 2d orientation in map space. Zero degrees and 360 degrees
/// are true north (NOT magnetic north; see physical_constants for
/// magnetic declination). Measures are clockwise, such that 90 degrees
/// is 3 o'clock and 270 is 9.
// We store in radians, for convenient usage in reads.
#[derive(Clone, Copy, Debug)]
pub struct Bearing(Angle<Radians>);

impl Bearing {
    pub fn north() -> Self {
        Self(radians!(0))
    }
    pub fn south() -> Self {
        Self(radians!(PI))
    }
    pub fn east() -> Self {
        Self(radians!(PI / 2.))
    }
    pub fn west() -> Self {
        Self(radians!(3. * PI / 2.))
    }

    pub fn new<Unit: AngleUnit>(angle: Angle<Unit>) -> Self {
        Self(radians!(angle))
    }

    pub fn angle<Unit: AngleUnit>(&self) -> Angle<Unit> {
        Angle::<Unit>::from(&self.0)
    }

    pub fn angle_mut(&mut self) -> &mut Angle<Radians> {
        &mut self.0
    }

    pub fn dvec2(&self) -> DVec2 {
        DVec2::new(self.0.sin().f64(), self.0.cos().f64())
    }

    pub fn dvec3(&self, pitch: PitchCline) -> DVec3 {
        let r = pitch.run();
        DVec3::new(self.0.sin().f64() * r, self.0.cos().f64() * r, pitch.rise())
    }

    pub fn dvec3_at_geo(&self, pitch: PitchCline, geode: &Geode) -> DVec3 {
        // let r_lon = quat_from_axis_angle(vec3(0., 1., 0.), lon);
        // let lat_axis = quat_rotate(r_lon, vec3(-1., 0., 0.)).v.xyz;
        // let r_lat = quat_from_axis_angle(lat_axis, (PI / 2.) - lat);
        // var ground_normal_w = quat_rotate(r_lat, quat_rotate(r_lon, ground_normal_local).v.xyz).v.xyz;
        let to_geo = DQuat::from_euler(
            EulerRot::YXZ,
            -geode.lon::<Radians>().f64(),
            -geode.lat::<Radians>().f64(),
            0.,
        );
        to_geo * self.dvec3(pitch)
    }

    pub fn interpolate(&self, other: &Self, f: f64) -> Self {
        // FIXME: this deals poorly with the singularity
        Self(self.0 + scalar!(f) * (other.0 - self.0))
    }
}

impl Add<Bearing> for Bearing {
    type Output = Self;

    fn add(self, rhs: Bearing) -> Self::Output {
        Bearing::new(self.0 + rhs.0)
    }
}

// impl From<Bearing> for Dict {
//     fn from(value: Bearing) -> Self {
//         let mut out = Dict::default();
//         out.insert("type", Value::from_str("Bearing"));
//         out.insert("angle", Value::from_float(value.angle::<Degrees>().f64()));
//         out
//     }
// }
//
// impl TryFrom<&Dict> for Bearing {
//     type Error = Error;
//
//     fn try_from(value: &Dict) -> Result<Self> {
//         let ty = value.expect_str("type", "Bearing")?;
//         ensure!(ty == "Bearing");
//         let angle = value.expect_float("angle", "Bearing")?;
//         Ok(Self::new(degrees!(angle)))
//     }
// }

impl fmt::Display for Bearing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;

    const TESTS: [(f64, (f64, f64)); 8] = [
        (0_f64, (0_f64, 1_f64)),
        (90_f64, (1_f64, 0_f64)),
        (180_f64, (0_f64, -1_f64)),
        (270_f64, (-1_f64, 0_f64)),
        (360_f64, (0_f64, 1_f64)),
        (-90_f64, (-1_f64, 0_f64)),
        (-180_f64, (0_f64, -1_f64)),
        (-270_f64, (1_f64, 0_f64)),
    ];

    #[test]
    fn test_bearing_to_angle() {
        for (angle, (x, y)) in &TESTS {
            let bearing = Bearing::new(degrees!(*angle));
            assert_relative_eq!(bearing.dvec2().x, x, epsilon = 1e15);
            assert_relative_eq!(bearing.dvec2().y, y, epsilon = 1e15);
        }
    }
}
