use std::mem;
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
use anyhow::{Result, bail, ensure};
use packed_struct::packed_struct;
use zerocopy::IntoBytes;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TiffType {
    Byte = 1,       // u8
    Ascii = 2,      // 7-bit chars with trailing \0
    Short = 3,      // u16
    Long = 4,       // u32
    Rational = 5,   // two longs, numerator followed by denominator
    SByte = 6,      // i8
    Undefined = 7,  // u8
    SShort = 8,     // i16
    SLong = 9,      // i32
    SRational = 10, // signed long version of rational
    Float = 11,     // f32
    Double = 12,    // f64
    Long8 = 16,     // u64
    SLong8 = 17,    // i64
    IFD8 = 18,      // IFD file offset
}

impl TryFrom<u16> for TiffType {
    type Error = anyhow::Error;
    fn try_from(v: u16) -> Result<Self> {
        Ok(match v {
            1 => Self::Byte,
            2 => Self::Ascii,
            3 => Self::Short,
            4 => Self::Long,
            5 => Self::Rational,
            6 => Self::SByte,
            7 => Self::Undefined,
            8 => Self::SShort,
            9 => Self::SLong,
            10 => Self::SRational,
            11 => Self::Float,
            12 => Self::Double,
            16 => Self::Long8,
            17 => Self::SLong8,
            18 => Self::IFD8,
            _ => bail!(format!("not a known tiff type tag: {v}")),
        })
    }
}

impl TiffType {
    // Size in bytes of an item of the given type.
    pub(crate) fn size_of(&self) -> usize {
        match self {
            Self::Byte => 1,
            Self::Ascii => 1,
            Self::Short => 2,
            Self::Long => 4,
            Self::Rational => 8,
            Self::SByte => 1,
            Self::Undefined => 1,
            Self::SShort => 2,
            Self::SLong => 4,
            Self::SRational => 8,
            Self::Float => 4,
            Self::Double => 8,
            Self::Long8 => 8,
            Self::SLong8 => 8,
            Self::IFD8 => 8,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum TiffTagId {
    NewSubfileType = 254,
    ImageWidth = 256,
    ImageLength = 257,
    BitsPerSample = 258,
    Compression = 259,
    PhotometricInterpretation = 262, // 1 = BlackIsZero, 2 = RGB
    SamplesPerPixel = 277,
    PlanarConfiguration = 284, // 1 = contiguous samples
    TileWidth = 322,
    TileLength = 323,
    TileOffsets = 324,
    TileByteCounts = 325,
    SampleFormat = 339,
    ModelPixelScale = 33550,
    ModelTiePoint = 33922,
    GeoKeyDirectoryTag = 34735,
    GdalMetadata = 42112,
}

impl TryFrom<u16> for TiffTagId {
    type Error = anyhow::Error;
    fn try_from(v: u16) -> Result<Self> {
        Ok(match v {
            254 => Self::NewSubfileType,
            256 => Self::ImageWidth,
            257 => Self::ImageLength,
            258 => Self::BitsPerSample,
            259 => Self::Compression,
            262 => Self::PhotometricInterpretation,
            277 => Self::SamplesPerPixel,
            284 => Self::PlanarConfiguration,
            322 => Self::TileWidth,
            323 => Self::TileLength,
            324 => Self::TileOffsets,
            325 => Self::TileByteCounts,
            339 => Self::SampleFormat,
            33550 => Self::ModelPixelScale,
            33922 => Self::ModelTiePoint,
            34735 => Self::GeoKeyDirectoryTag,
            42112 => Self::GdalMetadata,
            _ => bail!(format!("not a known tiff tag kind: {v}")),
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum GeoTiffKeyId {
    GtModel = 1024,
    GtRaster = 1025,
    Geographic = 2048,
}

impl TryFrom<u16> for GeoTiffKeyId {
    type Error = anyhow::Error;
    fn try_from(v: u16) -> Result<Self> {
        Ok(match v {
            1024 => Self::GtModel,
            1025 => Self::GtRaster,
            2048 => Self::Geographic,
            _ => bail!(format!("not a know geotiff key id: {v}")),
        })
    }
}

#[packed_struct]
pub(crate) struct BigTiffFileHeader {
    magic: [u8; 2],
    version: u16,
    bytesize_of_offsets: u16,
    always_zero: u16,
    offset_to_first_ifd: u64,
}

impl BigTiffFileHeader {
    pub(crate) fn validate_support(&self) -> Result<()> {
        ensure!(
            self.magic()[0] == 0x49,
            "Tiff only supports big-endian alignment currently"
        );
        ensure!(
            self.magic()[1] == 0x49,
            "Did not find a tiff file header at 1"
        );
        ensure!(
            self.version() == 43,
            "Tiff only supports header version 43 (bigtiff) currently"
        );
        ensure!(
            self.bytesize_of_offsets() == 8,
            "Tiff only supports 64 bit offsets currently"
        );
        ensure!(
            self.always_zero() == 0x0,
            "Tiff header expected 0 at always_zero"
        );
        Ok(())
    }
}

#[packed_struct]
pub(crate) struct BigIFDHeader {
    tag_count: u64,
}

#[packed_struct]
pub(crate) struct BigIFDFooter {
    next_ifd: u64,
}

#[packed_struct]
pub(crate) struct BigIFDTag {
    id: u16,
    field_type: u16,
    count: u64,
    inline_data: u64,
}

#[packed_struct]
pub(crate) struct GeoTiffModelPixelScale {
    x: f64,
    y: f64,
    z: f64,
}

#[packed_struct]
pub(crate) struct GeoTiffTiePoint {
    i: f64,
    j: f64,
    k: f64,
    x: f64,
    y: f64,
    z: f64,
}

#[packed_struct]
pub(crate) struct GeoTiffDirHeader {
    version: u16,
    key_revision: u16,
    minor_revision: u16,
    number_of_keys: u16,
}

#[packed_struct]
pub(crate) struct GeoTiffDirKey {
    id: u16,
    tiff_tag_location: u16,
    count: u16,
    value_offset: u16,
}

// From the TIFF 6.0 spec.
// To save time and space the Value Offset contains the Value instead of pointing to
// the Value if and only if the Value fits into 4 bytes. I
#[derive(Clone, Copy, Debug)]
pub(crate) enum OffsetOrInlineData {
    Offset(u64),
    // Note: we track the actual type to get the correct alignment.
    //       Trying to throw a slice on top of a byte slice stored in
    //       an enum will always fail as the header in the enum will
    //       ensure we're sitting at an odd offset.
    Data4([u32; 2]),
    Data8([u64; 1]),
}

#[derive(Debug)]
pub(crate) struct IFDTag {
    pub(crate) id: TiffTagId,
    pub(crate) field_type: TiffType,
    pub(crate) count: u64,
    pub(crate) inline_data: u64,
}

impl IFDTag {
    pub(crate) fn from_big(tag: &BigIFDTag) -> Result<Self> {
        let field_type = TiffType::try_from(tag.field_type())?;
        let inline_data = match field_type {
            TiffType::Float | TiffType::Long | TiffType::SLong => tag.inline_data & 0xFFFF_FFFF,
            TiffType::Short | TiffType::SShort => tag.inline_data & 0xFFFF,
            TiffType::Ascii | TiffType::Byte | TiffType::SByte => tag.inline_data & 0xFF,
            _ => tag.inline_data,
        };
        Ok(Self {
            id: TiffTagId::try_from(tag.id())?,
            field_type,
            count: tag.count(),
            inline_data,
        })
    }

    pub(crate) fn data_pointer(&self) -> Result<OffsetOrInlineData> {
        Ok(if self.data_size()? <= mem::size_of::<u64>() {
            match self.field_type {
                TiffType::Long => {
                    // Note: we should be able to Ref the inline data, but the type system is
                    //       claiming we're leaking the ref, even though we're not.
                    //       Instead just do the trivial copy here manually.
                    let b = self.inline_data.as_bytes();
                    let mut b0 = [0u8; 4];
                    b0.copy_from_slice(&b[0..4]);
                    let mut b1 = [0u8; 4];
                    b1.copy_from_slice(&b[4..8]);
                    OffsetOrInlineData::Data4([u32::from_le_bytes(b0), u32::from_le_bytes(b1)])
                }
                TiffType::Long8 => OffsetOrInlineData::Data8([self.inline_data]),
                _ => bail!("unsupported type of inline data {:?}", self.field_type),
            }
        } else {
            OffsetOrInlineData::Offset(self.inline_data)
        })
    }

    pub(crate) fn data_size(&self) -> Result<usize> {
        Ok(usize::try_from(self.count)? * self.field_type.size_of())
    }

    pub(crate) fn find(id: TiffTagId, tags: &[Self]) -> Option<&IFDTag> {
        tags.iter().find(|&tag| tag.id == id)
    }
}
