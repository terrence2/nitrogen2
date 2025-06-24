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
use anyhow::{Context, Result, ensure};
use progress_streams::{ProgressReader, ProgressWriter};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};
use ureq::Agent;

#[derive(Clone, Debug)]
pub(crate) struct TiffHttpReader {
    url: String,
    agent: Agent,
}

impl TiffHttpReader {
    pub(crate) fn new(url: &str) -> Result<Self> {
        let agent = ureq::AgentBuilder::new().user_agent("nitrogen/0.1").build();
        Ok(Self {
            url: url.to_owned(),
            agent,
        })
    }

    pub(crate) fn read_to_cache<R, W>(
        &self,
        start: u64,
        nbytes: usize,
        read_notify: R,
        write_notify: W,
        cache_fp: &mut File,
    ) -> Result<()>
    where
        R: Fn(usize),
        W: FnMut(usize),
    {
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

        cache_fp.seek(SeekFrom::Start(0))?;
        {
            let mut writer = ProgressWriter::new(&mut *cache_fp, write_notify);
            writer.write_all(&buf).with_context(|| {
                format!(
                    "create cache from {} at {start:016X}<-{nbytes:08X}",
                    self.url
                )
            })?;
        }
        cache_fp.seek(SeekFrom::Start(0))?;

        Ok(())
    }
}
