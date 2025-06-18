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
use anyhow::{Context, Result};
use parking_lot::RwLock;
use progress_streams::{ProgressReader, ProgressWriter};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    ops::DerefMut,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub(crate) struct TiffFileReader {
    url: String,
    fp: Arc<RwLock<File>>,
    end_offset: u64,
}

impl TiffFileReader {
    pub(crate) fn new(url: &str) -> Result<Self> {
        let mut fp =
            File::open(url).with_context(|| "failed to open local height cog file".to_string())?;
        let end_offset = fp.seek(SeekFrom::End(0))?;
        let start_offset = fp.seek(SeekFrom::Start(0))?;
        debug_assert_eq!(start_offset, 0);
        Ok(Self {
            url: url.to_owned(),
            fp: Arc::new(RwLock::new(fp)),
            end_offset,
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
        assert!(start < self.end_offset, "read past end of {}", self.url);
        assert!(
            (nbytes as u64) < self.end_offset,
            "read len longer than {}",
            self.url
        );

        // We can't use io::copy here; the read offset is stored behind the
        // int fd, so we need to hold the lock for the full read, effectively
        // serializing ourselves on the file access.
        let mut fp = self.fp.write();
        let fp_holder: &mut File = fp.deref_mut();
        let mut buf = vec![0; nbytes];
        fp_holder
            .seek(SeekFrom::Start(start))
            .with_context(|| format!("seek on fp in {} in tile mapper", self.url))?;
        {
            let mut reader = ProgressReader::new(fp_holder, read_notify);
            reader.read_exact(&mut buf).with_context(|| {
                format!("read source {} at {start:016X}<-{nbytes:08X}", self.url)
            })?;
        }
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
