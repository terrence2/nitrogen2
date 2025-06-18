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
use log::trace;
use std::path::Path;
// use ureq::Agent;

// Hide Mmap from higher layers so we can share with non-desktop platforms.
#[derive(Debug)]
pub(crate) struct Memory(Vec<u8>);

impl AsRef<[u8]> for Memory {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

// Expose tiff content. This layer is agnostic to tiff format.
#[derive(Clone, Debug)]
pub(crate) struct TiffAccess {
    url: String,
    // agent: Agent,
}

impl TiffAccess {
    pub(crate) fn new(_cache_dir: Option<&Path>, cog_url: &str) -> Result<Self> {
        trace!("TiffReader: {:?} into <null>", cog_url);
        assert!(
            cog_url.starts_with("http"),
            "only http tiff sources supported on web"
        );
        // let agent = ureq::AgentBuilder::new().user_agent("nitrogen/0.1").build();
        Ok(TiffAccess {
            url: cog_url.to_owned(),
            // agent,
        })
    }

    // Cache the range locally and return an mmap of the cached contents.
    pub(crate) fn mmap_cache<R, W>(
        &self,
        start: u64,
        nbytes: usize,
        read_notify: R,
        write_notify: W,
    ) -> Result<Memory>
    where
        R: Fn(usize),
        W: FnMut(usize),
    {
        self.get(start, nbytes, read_notify, write_notify)
            .map(Memory)
    }

    pub(crate) fn get<R, W>(
        &self,
        start: u64,
        nbytes: usize,
        _read_notify: R,
        _write_notify: W,
    ) -> Result<Vec<u8>>
    where
        R: Fn(usize),
        W: FnMut(usize),
    {
        trace!("TiffReader::get: {}+{} from {}", start, nbytes, self.url,);

        // FIXME: implement this!
        /*
        assert!(nbytes > 1);
        let end = start + nbytes as u64 - 1;
        assert_ne!(start, end);
        let res = self
            .agent
            .get(&self.url)
            .set("range", &format!("bytes={}-{}", start, end))
            .call()?;
        ensure!(
            res.status() == 206,
            "range requests return partial response"
        );

        let mut buf = vec![0; nbytes];
        let mut reader = ProgressReader::new(res.into_reader(), read_notify);
        reader
            .read_exact(&mut buf)
            .with_context(|| format!("read source {} at {start:016X}<-{nbytes:08X}", self.url))?;

        Ok(buf)
         */
        Ok(vec![])
    }
}
