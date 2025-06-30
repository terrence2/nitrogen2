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

///
/// Coordinate Systems:
///
/// World Space
/// -----------
/// Nitrogen uses a left handed coordinate system for planetary bodies.
///   +X -> negative longitudes, e.g. towards Americas (-X towards Asia)
///   +Y -> positive latitude, North Pole
///   +Z -> 0 degrees longitude, London
///
/// Geode(tic) latitude and longitude are as for Earth, with negative longitude going West
/// from +Z and positive longitude going East. Similarly, positive latitude goes North and
/// negative latitude goes South. These coordinates have a singularity at the poles and at
/// +/- 180.
///
/// Model Space
/// -----------
/// Nitrogen uses a left handed model space, similar to world space.
///   +X -> right
///   +Y -> up
///   +Z -> forward
///
/// Orientation of a model in world space is defined by a quaternion that transforms
/// +Z in model space to wherever that should be pointed in the world space; this is
/// the "facing". This is extremely useful for implementing, but is hard to reason
/// about or work with for humans, so we have:
///
/// Map Space
/// ---------
/// Nitrogen specifies and reports orientations in map space. In this space, north
/// is bearing 0 or 360 and bearings go clockwise with 90 and 3 o'clock and 270 at 9.
/// Pitch in this space, on the other hand is level (on the map, not in the world) is
/// at zero degrees, with positive being up and negative pointed down. Since going
/// past +/-90 would invert the bearing, pitches are limited to this range.
/// Similarly, roll is level at 0 degrees, but with a range of +/-180.
///
pub(crate) mod bearing;
pub(crate) mod geode;
pub(crate) mod geode_bb;
pub(crate) mod geodetic;
pub(crate) mod geodetic_bb;
pub(crate) mod pitch_cline;

pub use crate::{
    bearing::Bearing, geode::Geode, geode_bb::GeodeBB, geodetic::Geodetic, geodetic_bb::GeodeticBB,
    pitch_cline::PitchCline,
};

use absolute_unit::{Length, Meters};
pub const fn earth_radius() -> Length<Meters> {
    Length::<Meters>::new(6_356_766f64)
}

pub const fn everest_height() -> Length<Meters> {
    Length::<Meters>::new(8_848.039_2)
}
