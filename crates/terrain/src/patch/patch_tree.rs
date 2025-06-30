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
    common::{Handle, OptimizeCamera, Queue, UnorderedHeap},
    patch::{icosahedron::Icosahedron, patch_info::Patch, patch_winding::PatchWinding},
};
use absolute_unit::prelude::*;
use bevy::prelude::*;
use approx::assert_relative_eq;
use geometry::{Plane, algorithm::bisect_edge};
use glam::DVec3;
// use marker::Markers;
use ordered_float::OrderedFloat;
// use planck::{Color};
use geodesy::{earth_radius, everest_height};
use std::{cmp::Reverse, fmt::Write, time::Instant};

// It's possible that the current viewpoint just cannot be refined to our target, given
// our other constraints. Since we'll have another chance to optimize next frame, bail
// if we spend too much time optimizing.
const MAX_STUCK_ITERATIONS: usize = 3;

// Adds a sanity checks to the inner loop.
const PARANOIA_MODE: bool = false;

#[derive(Clone, Copy, Debug, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub(crate) struct PatchHeapTag;
pub(crate) type PatchHeap = UnorderedHeap<Patch, PatchHeapTag>;
pub(crate) type PatchHandle = Handle<PatchHeapTag>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct Peer {
    peer: PatchTreeHandle,
    opposite_edge: u8,
}

impl Peer {
    fn new(peer: PatchTreeHandle, opposite_edge: u8) -> Self {
        Self {
            peer,
            opposite_edge,
        }
    }

    fn from_icosahedron(raw: (usize, u8), root: &Root) -> Self {
        let (peer_offset, opposite_edge) = raw;
        let peer = root.children[peer_offset];
        assert!(opposite_edge < 3);
        Self {
            peer,
            opposite_edge,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Root {
    children: [PatchTreeHandle; 20],
}

#[derive(Copy, Clone, Debug)]
enum TreeHolder {
    Children([PatchTreeHandle; 4]),
    Patch(PatchHandle),
}

#[derive(Clone, Debug)]
pub(crate) struct PatchTreeNode {
    holder: TreeHolder,
    peers: [Option<Peer>; 3],
    parent: Option<PatchTreeHandle>,
    cached_solid_angle: f64,
    level: usize,
}

impl PatchTreeNode {
    pub(crate) fn is_leaf(&self) -> bool {
        !self.is_node()
    }

    fn is_node(&self) -> bool {
        match self.holder {
            TreeHolder::Children(ref _children) => true,
            TreeHolder::Patch(_) => false,
        }
    }

    pub(crate) fn children(&self) -> &[PatchTreeHandle; 4] {
        match self.holder {
            TreeHolder::Children(ref children) => children,
            TreeHolder::Patch(_) => panic!("called children on a leaf node"),
        }
    }

    pub(crate) fn patch_handle(&self) -> PatchHandle {
        match self.holder {
            TreeHolder::Children(_) => panic!("called patch_handle on a node"),
            TreeHolder::Patch(patch_handle) => patch_handle,
        }
    }

    fn solid_angle(&self) -> f64 {
        self.cached_solid_angle
    }

    fn peer(&self, edge: u8) -> &Option<Peer> {
        &self.peers[edge as usize]
    }

    fn peer_mut(&mut self, edge: u8) -> &mut Option<Peer> {
        &mut self.peers[edge as usize]
    }

    fn peers(&self) -> &[Option<Peer>; 3] {
        &self.peers
    }

    fn offset_of_child(&self, child_handle: PatchTreeHandle) -> u8 {
        for (i, child) in self.children().iter().enumerate() {
            if *child == child_handle {
                return i as u8;
            }
        }
        unreachable!("offset_of_child called with a non-child index")
    }
}

#[derive(Clone, Copy, Debug, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub(crate) struct PatchTreeTag;
pub(crate) type PatchTreeHeap = UnorderedHeap<PatchTreeNode, PatchTreeTag>;
pub(crate) type PatchTreeHandle = Handle<PatchTreeTag>;

// A virtual tree of patches. The tree recurses until a certain camera-relative "fineness"
// is achieved, defined by the max level, target refinement, and a patch count.
pub(crate) struct PatchTree {
    max_level: usize,
    target_refinement: f64,
    desired_patch_count: usize,

    patches: PatchHeap,
    tree: PatchTreeHeap,
    root: Root,

    split_queue: Queue<OrderedFloat<f64>, PatchTreeHandle>,
    merge_queue: Queue<Reverse<OrderedFloat<f64>>, PatchTreeHandle>,

    subdivide_count: usize,
    rejoin_count: usize,
    visit_count: usize,

    frame_number: usize,
    cached_viewable_region: [Plane<Meters>; 6],
    cached_eye_position: Pt3<Meters>,
    cached_eye_direction: DVec3,
    show_culling: bool,
}

impl PatchTree {
    pub(crate) fn new(
        max_level: usize,
        target_refinement: f64,
        desired_patch_count: usize,
    ) -> Self {
        let sphere = Icosahedron::new();
        let mut patches = PatchHeap::default();
        let mut tree = PatchTreeHeap::default();
        let mut roots = Vec::new();
        for face in sphere.faces.iter() {
            let v0 = (sphere.get_vert(face.i0()) * earth_radius()).pt3();
            let v1 = (sphere.get_vert(face.i1()) * earth_radius()).pt3();
            let v2 = (sphere.get_vert(face.i2()) * earth_radius()).pt3();
            let patch_handle = patches.alloc(Patch::new([v0, v1, v2]));
            roots.push(PatchTreeNode {
                level: 1,
                parent: None,
                holder: TreeHolder::Patch(patch_handle),
                peers: [None; 3],
                cached_solid_angle: -1f64,
            });
        }
        let handles = roots
            .drain(..)
            .map(|value| tree.alloc(value))
            .collect::<Vec<PatchTreeHandle>>();
        let root = Root {
            children: [
                handles[0],
                handles[1],
                handles[2],
                handles[3],
                handles[4],
                handles[5],
                handles[6],
                handles[7],
                handles[8],
                handles[9],
                handles[10],
                handles[11],
                handles[12],
                handles[13],
                handles[14],
                handles[15],
                handles[16],
                handles[17],
                handles[18],
                handles[19],
            ],
        };
        for (i, face) in sphere.faces.iter().enumerate() {
            tree.get_mut(root.children[i]).peers = [
                Some(Peer::from_icosahedron(face.sibling(0), &root)),
                Some(Peer::from_icosahedron(face.sibling(1), &root)),
                Some(Peer::from_icosahedron(face.sibling(2), &root)),
            ];
        }

        Self {
            max_level,
            target_refinement,
            desired_patch_count,
            patches,
            tree,
            root,
            subdivide_count: 0,
            rejoin_count: 0,
            visit_count: 0,
            frame_number: 0,
            split_queue: Queue::new(),
            merge_queue: Queue::new(),
            cached_viewable_region: [Plane::from_normal_and_distance(DVec3::X, meters!(0f64)); 6],
            cached_eye_position: Pt3::zero(),
            cached_eye_direction: DVec3::X,
            show_culling: false,
        }
    }

    pub(crate) fn toggle_show_culling(&mut self) {
        self.show_culling = !self.show_culling;
    }

    pub(crate) fn get_patch(&self, handle: PatchHandle) -> &Patch {
        self.patches.get(handle)
    }

    fn allocate_leaf(
        &mut self,
        parent_handle: PatchTreeHandle,
        level: usize,
        pts: [Pt3<Meters>; 3],
    ) -> PatchTreeHandle {
        let patch_handle = self.patches.alloc(Patch::new(pts));
        self.tree.alloc(PatchTreeNode {
            holder: TreeHolder::Patch(patch_handle),
            parent: Some(parent_handle),
            level,
            peers: [None; 3],
            cached_solid_angle: self.patches.get(patch_handle).compute_solid_angle(
                &self.cached_viewable_region,
                &self.cached_eye_position,
                &self.cached_eye_direction,
                &mut None,
            ),
        })
    }

    fn free_leaf(&mut self, leaf_handle: PatchTreeHandle) {
        assert!(
            self.tree.get(leaf_handle).is_leaf(),
            "trying to free non-leaf"
        );
        self.patches.free(self.tree.get(leaf_handle).patch_handle());
        self.tree.free(leaf_handle);
    }

    pub(crate) fn tree_patch(&self, tree_handle: PatchTreeHandle) -> &Patch {
        self.patches.get(self.tree.get(tree_handle).patch_handle())
    }

    pub(crate) fn solid_angle(&self, tree_handle: PatchTreeHandle) -> f64 {
        self.tree.get(tree_handle).solid_angle()
    }

    fn check_queues(&self) {
        self.split_queue.check_invariants();
        self.merge_queue.check_invariants();
        for tree_handle in self.split_queue.iter() {
            assert!(self.is_splittable_node(*tree_handle));
        }
        for tree_handle in self.merge_queue.iter() {
            assert!(self.is_mergeable_node(*tree_handle));
        }
    }

    fn max_splittable(&mut self) -> f64 {
        if self.split_queue.is_empty() {
            return f64::MIN;
        }
        self.split_queue.peek_value().0
    }

    pub(crate) fn is_splittable_node(&self, ti: PatchTreeHandle) -> bool {
        let node = self.tree.get(ti);
        node.is_leaf() && node.level < self.max_level
    }

    fn min_mergeable(&mut self) -> f64 {
        if self.merge_queue.is_empty() {
            return f64::MAX;
        }
        self.merge_queue.peek_value().0.0
    }

    fn is_leaf_node(&self, tree_handle: PatchTreeHandle) -> bool {
        let node = self.tree.get(tree_handle);
        node.is_node()
            && node
                .children()
                .iter()
                .all(|child| self.tree.get(*child).is_leaf())
    }

    pub(crate) fn is_mergeable_node(&self, ti: PatchTreeHandle) -> bool {
        let node = self.tree.get(ti);
        if node.is_leaf() {
            return false;
        }

        // Check if merging would cause us to violate constraints.
        // All of our peers must be either leafs or the adjacent children
        // must not be subdivided. If one of the adjacent nodes is subdivided
        // then merging this node would cause us to become adjacent to that
        // subdivided child node, which would be bad.
        //
        // Another way to state that would be that if any of our children's
        // external edges is Some and is more subdivided than the child,
        // then we cannot subdivide.
        for &child_handle in node.children() {
            let child = self.tree.get(child_handle);
            if !child.is_leaf() {
                return false;
            }

            for &peer_offset in &[0u8, 2u8] {
                if let Some(peer) = child.peer(peer_offset) {
                    if self.tree.get(peer.peer).is_node() {
                        return false;
                    }
                }
            }
        }

        true
    }

    pub(crate) fn optimize_for_view(
        &mut self,
        camera: &OptimizeCamera,
        live_patches: &mut Vec<(PatchHandle, PatchWinding)>,
        gizmos: &mut Option<&mut Gizmos>,
    ) {
        let reshape_start = Instant::now();

        self.subdivide_count = 0;
        self.rejoin_count = 0;
        self.visit_count = 0;

        self.cached_eye_position = camera.position::<Meters>();
        self.cached_eye_direction = *camera.forward();
        assert!(self.cached_eye_position.is_finite(), "camera 'sploded");

        // Take 5 culling planes from the frustum planes.
        for (i, f) in camera.world_space_frustum_m().iter().enumerate() {
            self.cached_viewable_region[i + 1] = *f;
        }
        // Replace the rear culling plane with the horizon culling plane
        let horizon_plane_height =
            earth_radius() * earth_radius() / self.cached_eye_position.length();
        self.cached_viewable_region[0] = Plane::from_normal_and_distance(
            self.cached_eye_position.dvec3().normalize(),
            horizon_plane_height,
        );

        if let Some(gizmos) = gizmos.as_deref_mut() {
            if self.show_culling {
                for i in 0..5 {
                    // FIXME: how best to visualize this with gizmos rather than markers?
                    // let plane = self.cached_viewable_region[i].nudged(meters!(-0.001));
                    // let to_xy = Quat::from_rotation_arc(plane.normal().as_vec3(), -Vec3::Z);
                    // gizmos
                    //     .primitive_3d(
                    //         &Plane3d::new(plane.normal().as_vec3(), Vec2::new(everest_height().f64() / 2., 10.)),
                    //         Isometry3d::new(self.cached_eye_position.dvec3().as_vec3(), to_xy),
                    //         GREEN,
                    //     )
                    //     .cell_count(UVec2::new(5, 10))
                    //     .spacing(Vec2::new(0.2, 0.1));
                    // markers.draw_plane_proxy(
                    //     Color::linear_rgb(0., 1., 0.),
                    //     &self.cached_eye_position,
                    //     &self.cached_viewable_region[i].nudged(meters!(-0.001)),
                    //     everest_height() * scalar!(1.),
                    // );
                }
                // markers.draw_plane_proxy(
                //     Color::linear_rgb(0., 1., 0.),
                //     &Pt3::zero(),
                //     &self.cached_viewable_region[5],
                //     everest_height() * scalar!(1.),
                // );
            }
        }

        // If f=0 {
        //   Let T = the base triangulation.
        //   Clear Qs, Qm.
        //   Compute priorities for T’s triangles and diamonds, then
        //     insert into Qs and Qm, respectively.
        if self.frame_number == 0 {
            for &child in &self.root.children {
                self.split_queue.insert(0f64.into(), child);
            }
        }
        self.frame_number += 1;

        // } otherwise {
        //   Continue processing T=T{f-1}.
        //   Update priorities for all elements of Qs, Qm.
        // }
        self.update_priorities(gizmos);

        // While T is not the target size/accuracy, or the maximum split priority is greater than the minimum merge priority {
        //   If T is too large or accurate {
        //      Identify lowest-priority (T, TB) in Qm.
        //      Merge (T, TB).
        //      Update queues as follows: {
        //        Remove all merged children from Qs.
        //        Add merge parents T, TB to Qs.
        //        Remove (T, TB) from Qm.
        //        Add all newly-mergeable diamonds to Qm.
        //   } otherwise {
        //     Identify highest-priority T in Qs.
        //     Force-split T.
        //     Update queues as follows: {
        //       Remove T and other split triangles from Qs.
        //       Add any new triangles in T to Qs.
        //       Remove from Qm any diamonds whose children were split.
        //       Add all newly-mergeable diamonds to Qm.
        //     }
        //   }
        // }
        // Set Tf = T.

        let target_patch_count = self.desired_patch_count;
        let mut stuck_iterations = 0;

        // While T is not the target size/accuracy, or the maximum split priority is greater than the minimum merge priority {
        while self.max_splittable() - self.min_mergeable() > self.target_refinement
            || self.patches.len() < target_patch_count - 4
            || self.patches.len() > target_patch_count
        {
            if self.patches.len() >= target_patch_count - 4
                && self.patches.len() <= target_patch_count
            {
                stuck_iterations += 1;
                if stuck_iterations > MAX_STUCK_ITERATIONS {
                    break;
                }
            }
            if PARANOIA_MODE {
                self.check_tree(None);
                self.check_queues();
            }

            //   If T is too large or accurate {
            if self.patches.len() >= target_patch_count {
                if self.merge_queue.is_empty() {
                    panic!("would merge but nothing to merge");
                }

                //      Identify lowest-priority (T, TB) in Qm.
                let bottom_key = self.merge_queue.pop();

                //      Merge (T, TB).
                self.rejoin_leaf_patch_into(bottom_key);
            } else {
                if self.split_queue.is_empty() {
                    panic!("would split but nothing to split");
                }

                // Identify highest-priority T in Qs.
                let top_leaf = self.split_queue.pop();

                // Force-split T.
                self.subdivide_leaf(top_leaf);
            }
        }
        assert!(
            self.patches.len() <= target_patch_count,
            "cached: {} of {}, stuck_iterations: {}, camera: {:?}",
            self.patches.len(),
            target_patch_count,
            stuck_iterations,
            camera
        );

        // Prepare for next frame.
        self.check_tree(None);
        self.check_queues();

        // Build a view-direction independent tesselation based on the current camera position.
        assert!(live_patches.is_empty());
        self.capture_patches(live_patches);
        assert!(live_patches.len() <= target_patch_count);
        let reshape_time = Instant::now() - reshape_start;

        // Select patches based on visibility.
        let max_split = self.max_splittable();
        let min_merge = self.min_mergeable();
        trace!(
            "i:{} r:{} qs:{} qm:{} p:{}/{} t:{}/{} | -/+: {}/{}/{} | {:.02}/{:.02} | {:?}",
            stuck_iterations,
            live_patches.len(),
            self.split_queue.len(),
            self.merge_queue.len(),
            self.patches.len(),
            self.patches.capacity(),
            self.tree.len(),
            self.tree.capacity(),
            self.rejoin_count,
            self.subdivide_count,
            self.visit_count,
            max_split,
            min_merge,
            reshape_time,
        );
    }

    fn pull_solid_angle(
        &mut self,
        tree_handle: &PatchTreeHandle,
        gizmos: &mut Option<&mut Gizmos>,
    ) -> f64 {
        let node = self.tree.get(*tree_handle);
        let mut sa = 0f64;
        match node.holder {
            TreeHolder::Patch(patch_handle) => {
                sa += self.patches.get(patch_handle).compute_solid_angle(
                    &self.cached_viewable_region,
                    &self.cached_eye_position,
                    &self.cached_eye_direction,
                    gizmos,
                );
            }
            TreeHolder::Children(children) => {
                for child in &children {
                    sa += self.pull_solid_angle(child, gizmos);
                }
            }
        }
        self.tree.get_mut(*tree_handle).cached_solid_angle = sa;
        sa
    }

    fn update_priorities(&mut self, gizmos: &mut Option<&mut Gizmos>) {
        // Visit all patches to apply the current view state
        let children = self.root.children;
        for tree_handle in &children {
            self.pull_solid_angle(tree_handle, gizmos);
        }

        // Re-sort split and merge queues, given the updated solid angles in the tree.
        let mut qs = self.split_queue.steal_content();
        for cache in qs.iter_mut() {
            if !self.split_queue.is_tombstoned(&cache.item) {
                cache.cache = self.solid_angle(cache.item).into();
            }
        }
        self.split_queue.restore_content(qs);

        let mut qm = self.merge_queue.steal_content();
        for cache in qm.iter_mut() {
            if !self.merge_queue.is_tombstoned(&cache.item) {
                cache.cache = Reverse(self.solid_angle(cache.item).into());
            }
        }
        self.merge_queue.restore_content(qm);
    }

    fn rejoin_leaf_patch_into(&mut self, tree_handle: PatchTreeHandle) {
        // println!(
        //     "MERGING {:?} w/ sa: {}",
        //     tree_handle,
        //     self.solid_angle(tree_handle)
        // );
        self.rejoin_count += 1;
        assert!(self.is_leaf_node(tree_handle));
        let children = *self.tree.get(tree_handle).children();

        // Save the corners for use in our new patch.
        let pts = [
            self.tree_patch(children[0]).points()[0],
            self.tree_patch(children[1]).points()[0],
            self.tree_patch(children[2]).points()[0],
        ];

        // Clear peer's backref links before we free the leaves.
        // Note: skip inner child links
        for &child in children.iter().take(3) {
            // Note: skip edges to inner child (always at 1 because 0 vertex faces outwards)
            for j in &[0u8, 2u8] {
                if let Some(child_peer) = self.tree.get(child).peer(*j) {
                    assert!(
                        self.tree.get(child_peer.peer).is_leaf(),
                        "Split should have removed peer parents from mergeable status when splitting."
                    );
                }
                self.update_peer_reverse_pointer(*self.tree.get(child).peer(*j), None);
            }
        }
        // Free children and remove from Qs.
        for &child in &children {
            self.free_leaf(child);
            //        Remove all merged children from Qs.
            self.split_queue.remove(child);
        }
        //        Remove (T, TB) from Qm.
        self.merge_queue.remove(tree_handle);

        // Replace the current node patch as a leaf patch and free the prior leaf node.
        let patch_handle = self.patches.alloc(Patch::new(pts));
        self.tree.get_mut(tree_handle).holder = TreeHolder::Patch(patch_handle);

        //        Add merge parents T, TB to Qs.
        self.split_queue
            .insert(self.solid_angle(tree_handle).into(), tree_handle);

        //        Add all newly-mergeable diamonds to Qm.
        if let Some(parent_handle) = self.tree.get(tree_handle).parent {
            if self.is_mergeable_node(parent_handle) {
                self.merge_queue.insert(
                    Reverse(self.solid_angle(parent_handle).into()),
                    parent_handle,
                );
            }
            for i in 0..3 {
                if let Some(peer) = self.tree.get(parent_handle).peers[i] {
                    if self.is_mergeable_node(peer.peer) {
                        self.merge_queue
                            .insert(Reverse(self.solid_angle(peer.peer).into()), peer.peer);
                    }
                }
            }
        }
    }

    fn subdivide_leaf(&mut self, tree_handle: PatchTreeHandle) {
        // println!(
        //     "SPLITTING {:?} w/ sa: {}",
        //     tree_handle,
        //     self.tree_patch(tree_handle).solid_angle()
        // );
        self.subdivide_count += 1;

        for own_edge_offset in 0u8..3u8 {
            if self.tree.get(tree_handle).peer(own_edge_offset).is_none() {
                if let Some(parent_handle) = self.tree.get(tree_handle).parent {
                    let parent = self.tree.get(parent_handle);
                    let offset_in_parent = parent.offset_of_child(tree_handle);
                    let parent_edge_offset =
                        self.find_parent_edge_for_child(own_edge_offset, offset_in_parent);
                    let parent_peer = parent
                        .peer(parent_edge_offset)
                        .expect("parent peer is absent")
                        .peer;

                    // If we have no peer, we're next to a larger triangle and need to subdivide it
                    // before moving forward.

                    // Note: the edge on self does not correspond to the parent edge.
                    let parent_peer_node = self.tree.get(parent_peer);
                    assert!(parent_peer_node.is_leaf());
                    self.subdivide_leaf(parent_peer);
                }
            }
        }

        let node = self.tree.get(tree_handle);
        let patch = self.patches.get(node.patch_handle());

        assert!(node.is_leaf());
        let current_level = node.level;
        let next_level = current_level + 1;
        assert!(next_level <= self.max_level);
        let [v0, v1, v2] = patch.points().to_owned();
        let maybe_parent_handle = node.parent;
        let leaf_peers = *node.peers();

        // Get new points.
        let a = (bisect_edge(v0.dvec3(), v1.dvec3()).normalize() * meters!(earth_radius())).pt3();
        let b = (bisect_edge(v1.dvec3(), v2.dvec3()).normalize() * meters!(earth_radius())).pt3();
        let c = (bisect_edge(v2.dvec3(), v0.dvec3()).normalize() * meters!(earth_radius())).pt3();

        // Allocate geometry to new patches.
        let children = [
            self.allocate_leaf(tree_handle, next_level, [v0, a, c]),
            self.allocate_leaf(tree_handle, next_level, [v1, b, a]),
            self.allocate_leaf(tree_handle, next_level, [v2, c, b]),
            self.allocate_leaf(tree_handle, next_level, [c, a, b]),
        ];

        // Note: we can't fill out inner peer info until after we create children anyway, so just
        // do the entire thing as a post-pass.
        let child_peers = self.make_children_peers(&children, &leaf_peers);
        for (&child_handle, &peers) in children.iter().zip(&child_peers) {
            self.tree.get_mut(child_handle).peers = peers;
        }

        // Recompute the solid angle of our node, using our new leaves solid angles
        // Note: the parent solid angles will remain wrong. In theory this should help with
        //       stability, but I've no evidence. We could probably just set SA to 0 initially
        //       and still be fine, as it will update next frame.
        let solid_angle = children
            .iter()
            .map(|hdl| self.tree.get(*hdl).solid_angle())
            .sum::<f64>();

        // Tell our neighbors that we have moved in.
        // Note: skip inner child
        for i in 0..3 {
            // Note: skip edges to inner child (always at 1 because 0 vertex faces outwards)
            for j in &[0, 2] {
                let peer = Some(Peer::new(children[i], *j as u8));
                self.update_peer_reverse_pointer(child_peers[i][*j], peer);
            }
        }

        //   Remove T and other split triangles from Qs
        self.split_queue.remove(tree_handle);

        //   Add any new triangles in T to Qs.
        for &child in &children {
            if next_level < self.max_level {
                self.split_queue
                    .insert(self.tree.get(child).solid_angle().into(), child);
            }
        }

        // We can now be merged, since our children are leafs.
        //   Add all newly-mergeable diamonds to Qm.
        self.merge_queue
            .insert(Reverse(solid_angle.into()), tree_handle);

        // Transform our leaf/patch into a node and clobber the old patch.
        self.patches.free(self.tree.get(tree_handle).patch_handle());
        self.tree.get_mut(tree_handle).holder = TreeHolder::Children(children);

        // We can no longer merge the parent, since we're now a node instead of a leaf.
        // We can also no longer merge our parent's peers, since that would create
        // a two level split difference after splitting this node.
        //   Remove from Qm any diamonds whose children were split.
        if let Some(parent_handle) = maybe_parent_handle {
            self.merge_queue.remove(parent_handle);
            for i in 0..3 {
                if let Some(peer) = self.tree.get(parent_handle).peers[i] {
                    if !self.is_mergeable_node(peer.peer) {
                        self.merge_queue.remove(peer.peer);
                    }
                }
            }
        }
    }

    fn update_peer_reverse_pointer(
        &mut self,
        maybe_peer: Option<Peer>,
        new_reverse_peer: Option<Peer>,
    ) {
        if let Some(peer) = maybe_peer {
            *self.tree.get_mut(peer.peer).peer_mut(peer.opposite_edge) = new_reverse_peer;
        }
    }

    fn find_parent_edge_for_child(&self, child_edge_offset: u8, child_offset_in_parent: u8) -> u8 {
        // Note: must only be called when the peer at own_edge_offset is None. This implies
        // that the edge we are visiting is more subdivided than the adjacent peer.
        assert_ne!(
            child_offset_in_parent, 3,
            "inner child should never have null edges"
        );

        // The first edge is inner, so must always be at least as subdivided as the outer child.
        assert_ne!(child_edge_offset, 1, "found no-peer-edge on inside edge");

        // On the 0'th edge, the child offset in the parent lines up with the parent edges.
        // On the 2'nd edge, the child offset in the parent is on the opposite side, so offset 2.
        // Ergo: (child_edge_offset + child_offset_in_parent) % 3
        (child_edge_offset + child_offset_in_parent) % 3
    }

    fn child_inner_peer_of(&self, peer: PatchTreeHandle, opposite_edge: u8) -> Peer {
        Peer {
            peer,
            opposite_edge,
        }
    }

    fn make_children_peers(
        &self,
        children: &[PatchTreeHandle; 4],
        peers: &[Option<Peer>; 3],
    ) -> [[Option<Peer>; 3]; 4] {
        [
            [
                self.child_peer_of(peers[0], 1, 2),
                Some(self.child_inner_peer_of(children[3], 0)),
                self.child_peer_of(peers[2], 0, 0),
            ],
            [
                self.child_peer_of(peers[1], 1, 2),
                Some(self.child_inner_peer_of(children[3], 1)),
                self.child_peer_of(peers[0], 0, 0),
            ],
            [
                self.child_peer_of(peers[2], 1, 2),
                Some(self.child_inner_peer_of(children[3], 2)),
                self.child_peer_of(peers[1], 0, 0),
            ],
            [
                Some(self.child_inner_peer_of(children[0], 1)),
                Some(self.child_inner_peer_of(children[1], 1)),
                Some(self.child_inner_peer_of(children[2], 1)),
            ],
        ]
    }

    // Which child is adjacent depends on what edge of the peer is adjacent to us. Because we are
    // selecting children anti-clockwise and labeling edges anti-clockwise, we can start with a
    // base peer assuming edge 0 is adjacent and bump by the edge offset to get the child peer
    // offset. Yes, this *is* weird, at least until you spend several hours caffinated beyond safe
    // limits. There are likely more useful patterns lurking.
    //
    // Because the zeroth index of every triangle in a subdivision faces outwards, opposite edge
    // is going to be the same no matter what way the triangle is facing, thus we only need one
    // `child_edge` indicator, independent of facing.
    fn child_peer_of(
        &self,
        maybe_peer: Option<Peer>,
        base_child_handle: usize,
        child_edge: u8,
    ) -> Option<Peer> {
        if let Some(peer) = maybe_peer {
            let peer_node = self.tree.get(peer.peer);
            if peer_node.is_leaf() {
                return None;
            }
            let adjacent_child = (base_child_handle + peer.opposite_edge as usize) % 3;
            return Some(Peer {
                peer: peer_node.children()[adjacent_child],
                opposite_edge: child_edge,
            });
        }
        None
    }

    fn check_edge_consistency(
        &self,
        tree_handle: PatchTreeHandle,
        own_edge_offset: u8,
        edge: &Option<Peer>,
    ) {
        fn assert_points_relative_eq(p0: &Pt3<Meters>, p1: &Pt3<Meters>) {
            let epsilon = 0.000_000_000_001;
            assert_relative_eq!(p0.x(), p1.x(), max_relative = epsilon);
            assert_relative_eq!(p0.y(), p1.y(), max_relative = epsilon);
            assert_relative_eq!(p0.z(), p1.z(), max_relative = epsilon);
        }

        let own_node = self.tree.get(tree_handle);
        if let Some(peer) = edge {
            let other_node = self.tree.get(peer.peer);
            if own_node.is_leaf() && other_node.is_leaf() {
                let own_patch = self.tree_patch(tree_handle);
                let peer_patch = self.tree_patch(peer.peer);
                let (s0, s1) = own_patch.edge(own_edge_offset);
                let (p0, p1) = peer_patch.edge(peer.opposite_edge);
                assert_points_relative_eq(&s0, &p1);
                assert_points_relative_eq(&s1, &p0);
            } else {
                // TODO: could drill down into children to get points to match
            }
        } else {
            // If the edge is None, the matching parent edge must not be None. This would imply
            // that the edge is adjacent to something more than one level of subdivision away
            // from it and thus that we would not be able to find an appropriate index buffer
            // winding to merge the levels.
            if let Some(parent_handle) = self.tree.get(tree_handle).parent {
                let parent = self.tree.get(parent_handle);
                let offset_in_parent = parent.offset_of_child(tree_handle);
                let parent_edge_offset =
                    self.find_parent_edge_for_child(own_edge_offset, offset_in_parent);
                let parent_peer = parent.peer(parent_edge_offset);
                assert!(parent_peer.is_some(), "adjacent subdivision level overflow");
            }
        }
    }

    fn check_tree(&self, split_context: Option<PatchTreeHandle>) {
        // self.print_tree();
        let children = self.root.children; // Clone to avoid dual-borrow.
        for i in &children {
            let peers = self.tree.get(*i).peers;
            self.check_tree_inner(1, *i, &peers, split_context);
        }
    }

    fn check_tree_inner(
        &self,
        level: usize,
        tree_handle: PatchTreeHandle,
        peers: &[Option<Peer>; 3],
        split_context: Option<PatchTreeHandle>,
    ) {
        let node = self.tree.get(tree_handle);

        if let Some(parent_handle) = node.parent {
            let parent = self.tree.get(parent_handle);
            assert!(parent.is_node());
            assert!(
                parent
                    .children()
                    .iter()
                    .any(|child_handle| *child_handle == tree_handle)
            );
        }

        for (i, stored_peer) in peers.iter().enumerate() {
            assert_eq!(*node.peer(i as u8), *stored_peer);
            self.check_edge_consistency(tree_handle, i as u8, &peers[i]);
        }

        if node.is_node() {
            if self.is_mergeable_node(tree_handle) {
                if !self.merge_queue.contains(&tree_handle) {
                    println!("{tree_handle:?} is mergeable, so should be in merge_queue");
                    self.print_tree().unwrap();
                }
                assert!(self.merge_queue.contains(&tree_handle));
            }
            let children_peers = self.make_children_peers(node.children(), peers);
            for (child, child_peers) in node.children().iter().zip(&children_peers) {
                self.check_tree_inner(level + 1, *child, child_peers, split_context);
            }
        } else {
            assert!(level <= self.max_level);
            if self.is_splittable_node(tree_handle) && level < self.max_level {
                if !(self.split_queue.contains(&tree_handle) || Some(tree_handle) == split_context)
                {
                    println!(
                        "{:?} is splittable, so should be in split_queue; {} < {} w/ ctx {:?}",
                        tree_handle, level, self.max_level, split_context
                    );
                    self.print_tree().unwrap();
                }
                assert!(
                    self.split_queue.contains(&tree_handle) || Some(tree_handle) == split_context
                );
            }
        }
    }

    fn capture_patches(&self, live_patches: &mut Vec<(PatchHandle, PatchWinding)>) {
        // We have already applied visibility at this level, so we just need to recurse.
        let children = self.root.children; // Clone to avoid dual-borrow.
        for i in &children {
            self.capture_live_patches_inner(1, *i, live_patches);
        }
    }

    fn capture_live_patches_inner(
        &self,
        level: usize,
        tree_handle: PatchTreeHandle,
        live_patches: &mut Vec<(PatchHandle, PatchWinding)>,
    ) {
        let node = self.tree.get(tree_handle);

        if node.is_node() {
            for &child in node.children() {
                self.capture_live_patches_inner(level + 1, child, live_patches);
            }
        } else {
            // Don't split leaves past max level.
            assert!(level <= self.max_level);
            live_patches.push((node.patch_handle(), PatchWinding::from_peers(&node.peers)));
        }
    }

    #[allow(unused)]
    pub(crate) fn print_tree(&self) -> Result<()> {
        println!("{}", self.format_tree_display()?);
        Ok(())
    }

    #[allow(unused)]
    fn format_tree_display(&self) -> Result<String> {
        let mut out = String::new();
        out += "Root\n";
        for child in &self.root.children {
            out += &self.format_tree_display_inner(1, *child)?;
        }
        Ok(out)
    }

    #[allow(unused)]
    fn format_tree_display_inner(
        &self,
        lvl: usize,
        tree_handle: PatchTreeHandle,
    ) -> Result<String> {
        fn fmt_peer(mp: &Option<Peer>) -> String {
            if let Some(p) = mp {
                format!("{}", p.peer)
            } else {
                "x".to_owned()
            }
        }
        let node = self.tree.get(tree_handle);
        let mut out = String::new();
        let pad = "  ".repeat(lvl);
        if node.is_node() {
            writeln!(
                out,
                "{}Node @{}, [{},{},{}]",
                pad,
                tree_handle,
                fmt_peer(&node.peers[0]),
                fmt_peer(&node.peers[1]),
                fmt_peer(&node.peers[2]),
            )?;
            for child in node.children() {
                out += &self.format_tree_display_inner(lvl + 1, *child)?;
            }
        } else {
            writeln!(
                out,
                "{}Leaf @{:?}, peers: [{},{},{}]",
                pad,
                node.patch_handle(),
                fmt_peer(&node.peers[0]),
                fmt_peer(&node.peers[1]),
                fmt_peer(&node.peers[2]),
            )?;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod test {
    /*
    use super::*;
    use absolute_unit::{degrees, meters};
    use arcball::ArcBallController;
    use camera::ScreenCamera;
    use geodesy::{Bearing, Geodetic, PitchCline};
    use hifitime::Epoch;
    use mantle::{Core, Gpu, PhysicalSize};

    #[test]
    fn test_pathological() -> Result<()> {
        let runtime = Core::for_test()?;
        let mut tree = PatchTree::new(15, 150.0, 300);
        let mut live_patches = Vec::new();
        let mut camera = ScreenCamera::new(
            "test",
            degrees!(90),
            PhysicalSize::new(1920, 1080),
            meters!(0.1),
            runtime.resource::<Gpu>(),
        );
        let mut arcball = ArcBallController::default();
        arcball.set_bearing(Bearing::new(degrees!(0)));
        arcball.set_pitch(PitchCline::new(degrees!(89)));
        arcball.set_distance(meters!(4_000_000))?;

        arcball.set_target_imm_unsafe(Geodetic::new(degrees!(0), degrees!(0), meters!(2)));
        camera.update_frame(&arcball.world_space_frame(), &Epoch::now()?);
        live_patches.clear();
        tree.optimize_for_view(&OptimizeCamera::from(&camera), &mut live_patches, &mut None);

        arcball.set_target_imm_unsafe(Geodetic::new(degrees!(0), degrees!(180), meters!(2)));
        camera.update_frame(&arcball.world_space_frame(), &Epoch::now()?);
        live_patches.clear();
        tree.optimize_for_view(&OptimizeCamera::from(&camera), &mut live_patches, &mut None);

        arcball.set_target_imm_unsafe(Geodetic::new(degrees!(0), degrees!(0), meters!(2)));
        camera.update_frame(&arcball.world_space_frame(), &Epoch::now()?);
        live_patches.clear();
        tree.optimize_for_view(&OptimizeCamera::from(&camera), &mut live_patches, &mut None);
        Ok(())
    }

    #[test]
    fn test_zoom_in() -> Result<()> {
        let runtime = Core::for_test()?;
        let mut tree = PatchTree::new(15, 150.0, 300);
        let mut live_patches = Vec::new();
        let mut camera = ScreenCamera::new(
            "test",
            degrees!(90),
            PhysicalSize::new(1920, 1080),
            meters!(0.1),
            runtime.resource::<Gpu>(),
        );
        let mut arcball = ArcBallController::default();
        arcball.set_target_imm_unsafe(Geodetic::new(degrees!(0), degrees!(0), meters!(2)));

        const CNT: i64 = 40;
        for i in 0..CNT {
            arcball.set_bearing(Bearing::new(degrees!(0)));
            arcball.set_pitch(PitchCline::new(degrees!(89)));
            arcball.set_distance(meters!(4_000_000 - i * (4_000_000 / CNT)))?;
            camera.update_frame(&arcball.world_space_frame(), &Epoch::now()?);
            live_patches.clear();
            tree.optimize_for_view(&OptimizeCamera::from(&camera), &mut live_patches, &mut None);
        }

        Ok(())
    }

    #[test]
    fn test_fly_forward() -> Result<()> {
        let runtime = Core::for_test()?;
        let mut tree = PatchTree::new(15, 150.0, 300);
        let mut live_patches = Vec::new();
        let mut camera = ScreenCamera::new(
            "test",
            degrees!(90),
            PhysicalSize::new(1920, 1080),
            meters!(0.1),
            runtime.resource::<Gpu>(),
        );
        let mut arcball = ArcBallController::default();
        arcball.set_target_imm_unsafe(Geodetic::new(degrees!(0), degrees!(0), meters!(1000)));
        arcball.set_bearing(Bearing::new(degrees!(90)));
        arcball.set_pitch(PitchCline::new(degrees!(1)));
        arcball.set_distance(meters!(1_500_000))?;

        const CNT: i64 = 40;
        for i in 0..CNT {
            arcball.set_target_imm_unsafe(Geodetic::new(
                degrees!(0),
                degrees!(4 * i),
                meters!(1000),
            ));
            camera.update_frame(&arcball.world_space_frame(), &Epoch::now()?);
            live_patches.clear();
            tree.optimize_for_view(&OptimizeCamera::from(&camera), &mut live_patches, &mut None);
        }

        Ok(())
    }
     */
}
