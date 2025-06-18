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
use crate::{
    geotiff::{GeoTiffIndices, SampleInfo},
    MapKind, MapName, MAP_SIZE,
};
use absolute_unit::prelude::*;
use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use log::{error, trace};
use rayon::{ThreadPool, ThreadPoolBuilder};
use runtime::{report_err, RuntimeResource};
use smallvec::SmallVec;
use std::{collections::HashMap, sync::Arc};
use zerocopy::Ref;

// Internal tracking per map name.
#[derive(Clone, Debug)]
pub enum MapState {
    Loading,
    Ready {
        data: Arc<Vec<u8>>,
        last_use: (u64, u64),
    },
    Empty,
}

// Return a map load operation to the foreground
struct MapLoadResult {
    rt_state: (u64, u64),
    map: MapName,
    status: Result<Vec<u8>>,
}

// Hosts in-memory sets of tiles that have been requested be available.
//
// Its primary interface is behind a queue, since we cannot guarantee how
// soon any particular request can be satisfied. There are also synchronous
// requests that may be optionally satisfied, for requests where the data
// needed may be stale if it is not delivered immediately, like terrain
// collision data. Most data is for reading, with occasional updates as
// I/O completes. As such, we use RwLock heavily here.

#[derive(Debug)]
pub(crate) struct Lru {
    fin_send: Sender<MapLoadResult>,
    fin_recv: Receiver<MapLoadResult>,
    pool: ThreadPool,
    indices: GeoTiffIndices,
    tiles: HashMap<MapName, MapState>,
}

impl Lru {
    pub(crate) fn new() -> Result<Self> {
        let (fin_send, fin_recv) = crossbeam::channel::unbounded();
        let pool = ThreadPoolBuilder::default().build()?;

        Ok(Self {
            fin_send,
            fin_recv,
            pool,
            indices: GeoTiffIndices::new(),
            tiles: HashMap::new(),
        })
    }

    pub(crate) fn map_is_ready(&self, map: &MapName) -> bool {
        matches!(self.tiles.get(map), Some(MapState::Ready { .. }))
    }

    // Note: this always collapses to the *South* West corner.
    fn sample_nearest(data: &[i16], uv: &(f64, f64)) -> Length<Meters> {
        let x = (uv.0 * MAP_SIZE as f64) as usize;
        let y = (uv.1 * MAP_SIZE as f64) as usize;
        let offset = y * MAP_SIZE as usize + x;
        meters!(data[offset])
    }

    pub(crate) fn add_indices(&mut self, indices: GeoTiffIndices) {
        self.indices.extend(indices);
    }

    pub(crate) fn get_height_nearest(&self, info: &SampleInfo) -> Option<Length<Meters>> {
        match self.tiles.get(info.map_name()) {
            None => None,
            Some(MapState::Loading) => None,
            Some(MapState::Empty) => Some(meters!(0)),
            Some(MapState::Ready { data, .. }) => {
                // Note: we've already filtered by using the right layer.
                let data = Ref::<&[u8], [i16]>::from_bytes(data).unwrap();
                Some(Self::sample_nearest(&data, info.uv()))
            }
        }
    }

    pub(crate) fn blit_height_for_grid(
        &mut self,
        (lo, hi): (&SampleInfo, &SampleInfo),
        height: Length<Meters>,
    ) {
        debug_assert_eq!(lo.map_name(), hi.map_name());
        let Some(MapState::Ready { data, last_use }) = self.tiles.get(lo.map_name()) else {
            return;
        };

        let x0 = (lo.uv().0 * MAP_SIZE as f64) as usize;
        let y0 = (lo.uv().1 * MAP_SIZE as f64) as usize;
        let x1 = (hi.uv().0 * MAP_SIZE as f64) as usize;
        let y1 = (hi.uv().1 * MAP_SIZE as f64) as usize;

        // We've shared with the renderer, so we have to orphan the Arc and replace the data blob.
        let mut data = data.as_slice().to_owned();
        let mut words = Ref::<&mut [u8], [u16]>::from_bytes(&mut data).unwrap();

        // Note that the byte order for y is upside down, so y0->y1 is hi to lo.
        let h = height.f64() as u16;
        for row in y1..y0 {
            let off0 = row * MAP_SIZE as usize + x0;
            let off1 = row * MAP_SIZE as usize + x1;
            for offset in off0..off1 {
                words[offset] = h;
            }
        }

        self.tiles.insert(
            lo.map_name().to_owned(),
            MapState::Ready {
                data: Arc::new(data),
                last_use: *last_use,
            },
        );
    }

    pub(crate) fn make_available(
        &mut self,
        maps: &[MapName],
        rt: &RuntimeResource,
    ) -> SmallVec<[(MapName, MapState); 9]> {
        maps.iter()
            .map(|name| (*name, self.make_map_available(name, rt)))
            .collect::<SmallVec<_>>()
    }

    pub(crate) fn make_map_available(&mut self, name: &MapName, rt: &RuntimeResource) -> MapState {
        let next = match self.tiles.get(name) {
            None => {
                self.request_map(name, rt);
                MapState::Loading
            }
            Some(MapState::Loading) => MapState::Loading,
            Some(MapState::Empty) => MapState::Empty,
            Some(MapState::Ready { data, .. }) => MapState::Ready {
                data: data.to_owned(),
                last_use: rt.state(),
            },
        };
        self.tiles.insert(*name, next.clone());
        next
    }

    pub(crate) fn request_map(&self, name: &MapName, rt: &RuntimeResource) {
        trace!("requesting map {name} at {:?}", rt.state());
        debug_assert!(!self.tiles.contains_key(name));

        let name = *name;
        let fin_send = self.fin_send.clone();
        let rt_state = rt.state();
        if let Some((index, reader)) = self.indices.get(&name.tiff_index_index()) {
            let index = index.to_owned();
            let reader = reader.to_owned();

            self.pool.spawn(move || {
                let span = index.read().at_offset(&name);
                let status = reader
                    .get(span.0, span.1, |_| {}, |_| {})
                    .map(|data| {
                        if data.is_empty() {
                            data
                        } else {
                            match xz_wrapper::decompress(&data) {
                                Ok(data) => data,
                                Err(e) => {
                                    error!("failed to decompress: {e:#?}");
                                    panic!("failed to decompress: {e}");
                                }
                            }
                        }
                    })
                    .map(|v| {
                        if v.is_empty() || name.kind() != MapKind::Colors {
                            v
                        } else {
                            debug_assert_eq!(v.len(), 512 * 512 * 3);
                            let mut out = vec![0u8; 512 * 512 * 4];
                            for i in 0..512 * 512 {
                                out[i * 4] = v[i * 3];
                                out[i * 4 + 1] = v[i * 3 + 1];
                                out[i * 4 + 2] = v[i * 3 + 2];
                                out[i * 4 + 3] = 255;
                            }
                            out
                        }
                    });
                let result = MapLoadResult {
                    rt_state,
                    map: name,
                    status,
                };
                fin_send.send(result).ok();
            });
        }
    }

    pub(crate) fn realize_loads(&mut self, notifications: &mut Vec<MapName>) {
        while let Ok(rv) = self.fin_recv.try_recv() {
            let data = match rv.status {
                Ok(v) => v,
                Err(err) => {
                    report_err(err);
                    continue;
                }
            };
            // Note: tiles should already have the value as we added a loading flag.
            assert!(matches!(self.tiles[&rv.map], MapState::Loading));
            let value = if !data.is_empty() {
                MapState::Ready {
                    data: Arc::new(data),
                    last_use: rv.rt_state,
                }
            } else {
                MapState::Empty
            };
            notifications.push(rv.map);
            self.tiles.insert(rv.map, value);
        }
    }

    pub(crate) fn memory_cache_size(&self) -> usize {
        self.tiles
            .values()
            .map(|v| match v {
                MapState::Ready { data, .. } => data.len(),
                _ => 0,
            })
            .sum()
    }
}
