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
mod file_reader;
mod http_reader;

use anyhow::{Context, Result};
use file_reader::TiffFileReader;
use http_reader::TiffHttpReader;
use log::trace;
use memmap2::Mmap;
use std::{
    fs,
    fs::{File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
};
use zerocopy::{IntoBytes, Ref};

// Hide Mmap from higher layers so we can share with non-desktop platforms.
#[repr(align(2))]
#[derive(Debug)]
pub(crate) enum Memory {
    Mapping(Mmap),
    Inline4([u32; 2]),
    Inline8([u64; 1]),
}

impl Memory {
    pub(crate) fn as_u64_arr(&self) -> Ref<&[u8], [u64]> {
        match self {
            Self::Mapping(mmap) => {
                Ref::<&[u8], [u64]>::from_bytes(mmap.as_ref()).expect("slice into *u64")
            }
            Self::Inline8(data) => {
                Ref::<&[u8], [u64]>::from_bytes(data.as_bytes()).expect("slice alignment")
            }
            Self::Inline4(_) => {
                panic!("attempting to access a Long inline IFD as a Long8")
            }
        }
    }

    pub(crate) fn as_u32_arr(&self) -> Ref<&[u8], [u32]> {
        match self {
            Self::Mapping(mmap) => {
                Ref::<&[u8], [u32]>::from_bytes(mmap.as_ref()).expect("slice into *u32")
            }
            Self::Inline4(data) => {
                Ref::<&[u8], [u32]>::from_bytes(data.as_bytes()).expect("slice alignment")
            }
            Self::Inline8(_) => {
                panic!("attempting to access a Long8 inline IFD as a Long")
            }
        }
    }
}

#[derive(Clone, Debug)]
enum TiffSource {
    File(TiffFileReader),
    Http(TiffHttpReader),
}

impl TiffSource {
    fn from_url(url: &str) -> Result<Self> {
        Ok(if url.starts_with("http") {
            Self::Http(TiffHttpReader::new(url)?)
        } else {
            Self::File(TiffFileReader::new(url)?)
        })
    }
}

// Expose tiff content with caching. This layer is agnostic to tiff format.
#[derive(Clone, Debug)]
pub(crate) struct TiffAccess {
    source: TiffSource,
    cache_prefix: PathBuf,
    url: String,
}

impl TiffAccess {
    pub(crate) fn new(cache_dir: Option<&Path>, cog_url: &str) -> Result<Self> {
        let mut cache_prefix = cache_dir.expect("std dirs").to_owned();
        cache_prefix.push("cog");
        cache_prefix.push(format!("{:x}", md5::compute(cog_url)));
        fs::create_dir_all(&cache_prefix).with_context(|| "unable to create cog cache")?;
        trace!("TiffReader: {:?} into {:?}", cog_url, cache_prefix);

        Ok(TiffAccess {
            source: TiffSource::from_url(cog_url)?,
            cache_prefix,
            url: cog_url.to_owned(),
        })
    }

    fn ensure_cached<R, W>(
        &self,
        start: u64,
        nbytes: usize,
        read_notify: R,
        mut write_notify: W,
    ) -> Result<File>
    where
        R: Fn(usize),
        W: FnMut(usize),
    {
        let cache_path = self.cache_path(start, nbytes);
        let mut cache_fp = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&cache_path)
            .with_context(|| format!("failed to open cache file at {cache_path:?}"))?;

        // Check if we've already cached the file
        if let Ok(stat) = cache_fp.metadata() {
            if stat.len() == nbytes as u64 {
                read_notify(nbytes);
                write_notify(nbytes);
                return Ok(cache_fp);
            }
        }

        trace!(
            "TiffReader::ensure_cached: {}+{} from {} into {:?}",
            start,
            nbytes,
            self.url,
            cache_path
        );
        match &self.source {
            TiffSource::File(src) => {
                src.read_to_cache(start, nbytes, read_notify, write_notify, &mut cache_fp)?
            }
            TiffSource::Http(src) => {
                src.read_to_cache(start, nbytes, read_notify, write_notify, &mut cache_fp)?
            }
        };

        Ok(cache_fp)
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
        let cache_fp = self.ensure_cached(start, nbytes, read_notify, write_notify)?;
        unsafe { Mmap::map(&cache_fp) }
            .with_context(|| format!("mapping cache for {start:016X}<-{nbytes:08X}"))
            .map(Memory::Mapping)
    }

    pub(crate) fn get<R, W>(
        &self,
        start: u64,
        nbytes: usize,
        read_notify: R,
        write_notify: W,
    ) -> Result<Vec<u8>>
    where
        R: Fn(usize),
        W: FnMut(usize),
    {
        let mut cache_fp = self
            .ensure_cached(start, nbytes, read_notify, write_notify)
            .with_context(|| format!("ensure cached {start:016X}<-{nbytes:08X}"))?;
        let mut buf = Vec::new();
        cache_fp
            .read_to_end(&mut buf)
            .with_context(|| format!("cog cache {start:016X}<-{nbytes:08X}"))?;
        Ok(buf)
    }

    fn cache_path(&self, start: u64, nbytes: usize) -> PathBuf {
        let mut cache_path = self.cache_prefix.clone();
        cache_path.push(format!("{:016X}-{:08X}.dat", start, nbytes));
        cache_path
    }
}
