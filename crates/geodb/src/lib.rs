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
mod attachments;
mod cache_access;
mod geodb;
pub(crate) mod geotiff;
mod levels;
pub(crate) mod lru;
mod options;
mod tiff;

pub const MAP_SIZE: u32 = 512;

pub use crate::{
    attachments::{
        flatten::Flatten,
        foundation::Foundation,
        ground_stabilizer::GroundStabilizer,
        tarmac::{Tarmac, TarmacBuilding},
    },
    geodb::{GeoDb, GeoDbStep, MapKind, MapName},
    levels::{OverviewLevel, TileSubdivisionLevel},
    lru::MapState,
    options::GeoDbOpts,
};
