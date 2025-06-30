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
use glam::DVec3;

pub struct Face {
    pub indices: [u8; 3],
    pub siblings: [[u8; 2]; 3], // 0-1, 1-2, 2-0
}

impl Face {
    pub fn new(indices: [u8; 3], siblings: [[u8; 2]; 3]) -> Self {
        Face { indices, siblings }
    }

    pub fn i0(&self) -> usize {
        self.indices[0] as usize
    }

    pub fn i1(&self) -> usize {
        self.indices[1] as usize
    }

    pub fn i2(&self) -> usize {
        self.indices[2] as usize
    }

    #[allow(unused)]
    pub fn edge(&self, i: usize) -> [usize; 2] {
        match i {
            0 => [self.i0(), self.i1()],
            1 => [self.i1(), self.i2()],
            2 => [self.i2(), self.i0()],
            _ => unreachable!(),
        }
    }

    pub fn sibling(&self, i: usize) -> (usize, u8) {
        (self.siblings[i][0] as usize, self.siblings[i][1])
    }
}

pub struct Icosahedron {
    pub verts: Vec<DVec3>,
    pub faces: Vec<Face>,
}

impl Icosahedron {
    pub fn new() -> Self {
        let t = (1_f64 + 5_f64.sqrt()) / 2_f64;

        // The bones of the d12 are 3 orthogonal quads at the origin.
        let verts = vec![
            DVec3::new(-1_f64, t, 0_f64).normalize(),
            DVec3::new(1_f64, t, 0_f64).normalize(),
            DVec3::new(-1_f64, -t, 0_f64).normalize(),
            DVec3::new(1_f64, -t, 0_f64).normalize(),
            DVec3::new(0_f64, -1_f64, t).normalize(),
            DVec3::new(0_f64, 1_f64, t).normalize(),
            DVec3::new(0_f64, -1_f64, -t).normalize(),
            DVec3::new(0_f64, 1_f64, -t).normalize(),
            DVec3::new(t, 0_f64, -1_f64).normalize(),
            DVec3::new(t, 0_f64, 1_f64).normalize(),
            DVec3::new(-t, 0_f64, -1_f64).normalize(),
            DVec3::new(-t, 0_f64, 1_f64).normalize(),
        ];

        let faces = vec![
            // -- 5 faces around point 0
            /* 0 */
            Face::new([0, 11, 5], [[4, 2], [6, 0], [1, 0]]),
            /* 1 */ Face::new([0, 5, 1], [[0, 2], [5, 0], [2, 0]]),
            /* 2 */ Face::new([0, 1, 7], [[1, 2], [9, 0], [3, 0]]),
            /* 3 */ Face::new([0, 7, 10], [[2, 2], [8, 0], [4, 0]]),
            /* 4 */ Face::new([0, 10, 11], [[3, 2], [7, 0], [0, 0]]),
            // -- 5 adjacent faces
            /* 5 */
            Face::new([1, 5, 9], [[1, 1], [15, 1], [19, 2]]),
            /* 6 */ Face::new([5, 11, 4], [[0, 1], [16, 1], [15, 2]]),
            /* 7 */ Face::new([11, 10, 2], [[4, 1], [17, 1], [16, 2]]),
            /* 8 */ Face::new([10, 7, 6], [[3, 1], [18, 1], [17, 2]]),
            /* 9 */ Face::new([7, 1, 8], [[2, 1], [19, 1], [18, 2]]),
            // -- 5 faces around point 3
            /* 10 */
            Face::new([3, 9, 4], [[14, 2], [15, 0], [11, 0]]),
            /* 11 */ Face::new([3, 4, 2], [[10, 2], [16, 0], [12, 0]]),
            /* 12 */ Face::new([3, 2, 6], [[11, 2], [17, 0], [13, 0]]),
            /* 13 */ Face::new([3, 6, 8], [[12, 2], [18, 0], [14, 0]]),
            /* 14 */ Face::new([3, 8, 9], [[13, 2], [19, 0], [10, 0]]),
            // -- 5 adjacent faces
            /* 15 */
            Face::new([4, 9, 5], [[10, 1], [5, 1], [6, 2]]),
            /* 16 */ Face::new([2, 4, 11], [[11, 1], [6, 1], [7, 2]]),
            /* 17 */ Face::new([6, 2, 10], [[12, 1], [7, 1], [8, 2]]),
            /* 18 */ Face::new([8, 6, 7], [[13, 1], [8, 1], [9, 2]]),
            /* 19 */ Face::new([9, 8, 1], [[14, 1], [9, 1], [5, 2]]),
        ];

        Self { verts, faces }
    }

    pub fn get_vert(&self, i: usize) -> DVec3 {
        self.verts[i]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use absolute_unit::prelude::*;
    use geodesy::{Geodetic, earth_radius};

    #[test]
    fn icosahedron_peer_linkage() {
        let ico = Icosahedron::new();
        assert_eq!(ico.verts.len(), 12);
        assert_eq!(ico.faces.len(), 20);

        for (i, face) in ico.faces.iter().enumerate() {
            println!("at face: {i:?}");
            for (j, [sib, peer_edge, ..]) in face.siblings.iter().enumerate() {
                assert_eq!(
                    face.edge(j)[0],
                    ico.faces[*sib as usize].edge(*peer_edge as usize)[1]
                );
                assert_eq!(
                    face.edge(j)[1],
                    ico.faces[*sib as usize].edge(*peer_edge as usize)[0]
                );
            }
        }
    }

    #[test]
    fn icosahedron_geodetic() {
        let ico = Icosahedron::new();
        for i in 0..12 {
            let v0 = ico.get_vert(i);
            let p0 = v0 * meters!(earth_radius());
            let g0 = Geodetic::from(p0.pt3());
            if v0.x < 0. {
                assert!(g0.lon::<Degrees>() > degrees!(0));
            } else {
                assert!(g0.lon::<Degrees>() <= degrees!(0));
            }
        }
    }
}
