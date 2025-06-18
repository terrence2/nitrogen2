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

use absolute_unit::{Length, Meters};
use bevy_ecs::prelude::*;
use nitrous::{inject_nitrous_component, NitrousComponent};

/// Similar to a foundation, but for things that move. These will keep a relative height to
/// the ground if the ground moves in a way that will interact with an attached CollisionImpostor.
/// Unlike foundation, this component expects the entity to move.
#[derive(NitrousComponent, Default)]
#[component(name = "ground_stabilizer")]
pub struct GroundStabilizer {
    // Store the prior ground height so that we can adjust by the correct amount on loads.
    // No, that doesn't work. We'll have moved in the prior frame. Unless we want to do
    // a lookup before loading the terrain. I think we need to?
    current_height: Length<Meters>,
}

#[inject_nitrous_component]
impl GroundStabilizer {
    pub fn current_height(&self) -> Length<Meters> {
        self.current_height
    }

    pub fn set_current_height(&mut self, next: Length<Meters>) {
        self.current_height = next;
    }
}
