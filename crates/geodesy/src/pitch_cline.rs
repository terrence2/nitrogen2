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
use absolute_unit::prelude::*;
// use anyhow::{ensure, Error, Result};
// use nitrous::{Dict, Value};
use std::{fmt, ops::Neg};

/// Represents a 2d elevation (in map space, not that it matters).
/// Positive elevation is "up" and will produce positive rise numbers.
// We store in radians, for convenient usage in reads.
#[derive(Clone, Copy, Debug)]
pub struct PitchCline(Angle<Radians>);

impl PitchCline {
    pub fn level() -> Self {
        Self(radians!(0))
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

    pub fn rise(&self) -> f64 {
        self.0.sin().f64()
    }

    pub fn run(&self) -> f64 {
        self.0.cos().f64()
    }

    pub fn interpolate(&self, other: &Self, f: f64) -> Self {
        Self(self.0 + scalar!(f) * (other.0 - self.0))
    }
}

impl Neg for PitchCline {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::new(-self.0)
    }
}

// impl From<PitchCline> for Dict {
//     fn from(value: PitchCline) -> Self {
//         let mut out = Dict::default();
//         out.insert("type", Value::from_str("PitchCline"));
//         out.insert("angle", Value::from_float(value.angle::<Degrees>().f64()));
//         out
//     }
// }
//
// impl TryFrom<&Dict> for PitchCline {
//     type Error = Error;
//
//     fn try_from(value: &Dict) -> Result<Self> {
//         let ty = value.expect_str("type", "PitchCline")?;
//         ensure!(ty == "PitchCline");
//         let angle = value.expect_float("angle", "PitchCline")?;
//         Ok(Self::new(degrees!(angle)))
//     }
// }

impl fmt::Display for PitchCline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_pitch_to_rise_run() {
        const SC45: f64 = std::f64::consts::FRAC_1_SQRT_2;
        const TESTS: [(f64, (f64, f64)); 5] = [
            (0_f64, (0_f64, 1_f64)),
            (45_f64, (SC45, SC45)),
            (90_f64, (1_f64, 0_f64)),
            (-45_f64, (-SC45, SC45)),
            (-90_f64, (-1_f64, 0_f64)),
        ];
        for (angle, (x, y)) in &TESTS {
            let pitch = PitchCline::new(degrees!(*angle));
            assert_relative_eq!(pitch.rise(), *x);
            assert_relative_eq!(pitch.run(), *y);
        }
    }
}
