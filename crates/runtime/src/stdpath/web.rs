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
use crate::{Extension, Runtime};
use nitrous::{NitrousResource, inject_nitrous_resource};
use std::path::Path;

#[derive(Debug)]
pub struct StdPathsOpts {
    _name: String,
}

impl StdPathsOpts {
    pub fn new(name: &str) -> Self {
        Self {
            _name: name.to_owned(),
        }
    }
}

#[derive(NitrousResource)]
pub struct StdPaths {}

impl Extension for StdPaths {
    type Opts = StdPathsOpts;

    fn init(runtime: &mut Runtime, _opts: Self::Opts) -> anyhow::Result<()> {
        runtime.inject_resource(StdPaths {})?;
        Ok(())
    }
}

#[inject_nitrous_resource]
impl StdPaths {
    pub fn state_dir(&self) -> Option<&Path> {
        None
    }
}
