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
use crate::Segment;
use absolute_unit::{Length, LengthUnit};

// A capsule is a line segment with a radius around it.
// Most collisions against one boil down to a segment approach with a distance check.
#[derive(Clone, Debug)]
pub struct Capsule<Unit: LengthUnit> {
    segment: Segment<Unit>,
    radius: Length<Unit>,
}

impl<Unit: LengthUnit> Capsule<Unit> {
    pub fn new(segment: Segment<Unit>, radius: Length<Unit>) -> Self {
        Self { segment, radius }
    }

    pub fn segment(&self) -> &Segment<Unit> {
        &self.segment
    }

    pub fn radius(&self) -> &Length<Unit> {
        &self.radius
    }
}
