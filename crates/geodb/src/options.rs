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
use structopt::StructOpt;

#[derive(Clone, Debug, Default, StructOpt)]
pub struct GeoDbOpts {
    /// Override the terrain height map.
    #[structopt(long)]
    tiles_height_url: Option<String>,

    /// Override the terrain color map.
    #[structopt(long)]
    tiles_color_url: Option<String>,
}

impl GeoDbOpts {
    const COLORS_URL: &'static str =
        "https://storage.googleapis.com/openfa/bmng-jan-sparse-512-lzma.cog";
    const HEIGHTS_URL: &'static str =
        "https://storage.googleapis.com/openfa/srtm-sparse-512-lzma.cog";

    pub fn tiles_color_url(&self) -> &str {
        if let Some(ref url) = self.tiles_color_url {
            url
        } else {
            Self::COLORS_URL
        }
    }

    pub fn tiles_height_url(&self) -> &str {
        if let Some(ref url) = self.tiles_height_url {
            url
        } else {
            Self::HEIGHTS_URL
        }
    }
}
