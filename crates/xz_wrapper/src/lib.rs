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
use anyhow::Result;

// Note: enforce mut requirement on callers, even on platforms
//       that don't require this to be mut.
#[allow(unused_mut)]
pub fn decompress(mut buf: &[u8]) -> Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut _out = vec![];
        lzma_rs::xz_decompress(&mut buf, &mut _out)?;
        Ok(_out)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(lzma::decompress(buf)?)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn compress(buf: &[u8], preset: u32) -> Result<Vec<u8>> {
    Ok(lzma::compress(buf, preset)?)
}
