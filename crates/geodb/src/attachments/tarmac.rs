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
use bevy::prelude::*;

// A building that can be attached to a Tarmac
// FIXME: this functionality should be integrated with Foundation somehow to make matching easier
#[derive(Component)]
pub struct TarmacBuilding {
    offset_to_ground: Length<Meters>,
}

impl TarmacBuilding {
    pub fn new(offset_to_ground: Length<Meters>) -> Self {
        Self { offset_to_ground }
    }

    pub fn offset_to_ground(&self) -> Length<Meters> {
        self.offset_to_ground
    }
}

// A flattener specifically for an Airport. An Airport needs to be
// level in a region, but also needs to have a bunch of items placed
// on it in rectilinear coordinates rather than geodetic, as geodetic
// inaccuracy is significant at the lengths involved, at high
// latitudes.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Tarmac {
    offset_to_ground: Length<Meters>,
    buildings: Vec<Entity>,
}

impl Tarmac {
    pub fn new(offset_to_ground: Length<Meters>, buildings: &[Entity]) -> Self {
        Self {
            offset_to_ground,
            buildings: buildings.to_owned(),
        }
    }

    pub fn offset_to_ground(&self) -> Length<Meters> {
        self.offset_to_ground
    }

    pub(crate) fn buildings(&self) -> &[Entity] {
        &self.buildings
    }
}
