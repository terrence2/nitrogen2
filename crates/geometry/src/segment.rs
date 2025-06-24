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
use absolute_unit::{Length, LengthUnit, Pt3, V3, scalar};
use glam::DVec3;

#[derive(Clone, Debug)]
pub struct Segment<Unit: LengthUnit> {
    p0: Pt3<Unit>,
    p1: Pt3<Unit>,
}

impl<Unit: LengthUnit> Segment<Unit> {
    pub fn new(p0: Pt3<Unit>, p1: Pt3<Unit>) -> Self {
        Self { p0, p1 }
    }

    pub fn start(&self) -> &Pt3<Unit> {
        &self.p0
    }

    pub fn end(&self) -> &Pt3<Unit> {
        &self.p1
    }

    pub fn center(&self) -> Pt3<Unit> {
        self.p0 + self.direction() * (self.length() / scalar!(2))
    }

    pub fn length(&self) -> Length<Unit> {
        self.p0.to(self.p1).magnitude()
    }

    pub fn direction(&self) -> DVec3 {
        self.p0.to(self.p1).normalize()
    }

    pub fn direction_v3(&self) -> V3<Length<Unit>> {
        V3::new_dvec3(self.p0.to(self.p1).normalize())
    }
}
