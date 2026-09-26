use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;

use rstar::ParentNode;
use rstar::PointDistance as _;
use rstar::RTreeNode;
use rstar_geodetic::GeodeticPoint;
use rstar_geodetic::GeodeticRTree;
use rstar_geodetic::UnitVec;
use rstar_geodetic::indexed::IndexedPoint;

use crate::Candidate;
use crate::Filter;
use crate::Position;
use crate::gbfs::Vehicle;

/// The available vehicles of one feed, indexed for nearest-neighbour queries
/// on the sphere.
///
/// Build a new index for each feed: a GBFS feed is a full snapshot, and a bulk
/// load gives a better tree than a sequence of inserts and removals.
#[derive(Debug)]
pub struct VehicleIndex {
    tree: GeodeticRTree<IndexedPoint>,
    vehicles: Vec<IndexedVehicle>,
}

/// The candidates in an index, nearest to an origin first. It borrows the
/// index.
#[derive(Debug)]
pub struct Nearest<'a> {
    index: &'a VehicleIndex,
    /// The traversal, or `None` if the origin is not a valid position.
    cursor: Option<Cursor>,
}

/// The candidates in a shared index, nearest to an origin first. It owns a
/// reference to the index, so it can outlive the caller that made it.
#[derive(Debug)]
pub struct SharedNearest {
    index: Arc<VehicleIndex>,
    /// The traversal, or `None` if the origin is not a valid position.
    cursor: Option<Cursor>,
}

impl VehicleIndex {
    /// Indexes the vehicles that are available, have a position, and report
    /// a range.
    #[must_use]
    pub fn new(vehicles: &[Vehicle]) -> Self {
        let vehicles: Vec<IndexedVehicle> = vehicles
            .iter()
            .filter(|vehicle| vehicle.is_available())
            .filter_map(|vehicle| {
                let (lat, lon) = vehicle.position()?;
                Some(IndexedVehicle {
                    vehicle_id: vehicle.vehicle_id.clone(),
                    point: GeodeticPoint::try_new(lon, lat).ok()?,
                    position: Position::new(lat, lon),
                    range_m: vehicle.current_range_meters?,
                    ios_rental_uri: vehicle
                        .rental_uris
                        .as_ref()
                        .and_then(|uris| uris.ios.clone()),
                })
            })
            .collect();
        let leaves = vehicles
            .iter()
            .enumerate()
            .map(|(index, vehicle)| IndexedPoint::new(vehicle.point, index))
            .collect();
        Self {
            tree: GeodeticRTree::bulk_load(leaves),
            vehicles,
        }
    }

    /// Returns the vehicles that satisfy `filter`, nearest to `origin` first.
    /// If `origin` is not a valid position, the iterator is empty.
    #[must_use]
    pub fn nearest(&self, origin: Position, filter: Filter) -> Nearest<'_> {
        Nearest {
            index: self,
            cursor: Cursor::new(self, origin, filter),
        }
    }

    /// Returns the vehicles that satisfy `filter`, nearest to `origin` first.
    /// The iterator keeps a reference to the index. If `origin` is not a valid
    /// position, the iterator is empty.
    #[must_use]
    pub fn shared_nearest(self: &Arc<Self>, origin: Position, filter: Filter) -> SharedNearest {
        SharedNearest {
            index: Arc::clone(self),
            cursor: Cursor::new(self, origin, filter),
        }
    }

    /// Returns the number of vehicles in the index.
    #[must_use]
    pub fn len(&self) -> usize {
        self.vehicles.len()
    }

    /// Returns `true` if the index has no vehicles.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vehicles.is_empty()
    }

    fn root(&self) -> &ParentNode<IndexedPoint> {
        self.tree.root()
    }
}

impl Iterator for Nearest<'_> {
    type Item = Candidate;

    fn next(&mut self) -> Option<Candidate> {
        self.cursor.as_mut()?.next(self.index)
    }
}

impl Iterator for SharedNearest {
    type Item = Candidate;

    fn next(&mut self) -> Option<Candidate> {
        self.cursor.as_mut()?.next(&self.index)
    }
}

#[derive(Clone, Debug)]
struct IndexedVehicle {
    vehicle_id: String,
    point: GeodeticPoint,
    position: Position,
    range_m: f64,
    ios_rental_uri: Option<String>,
}

/// A best-first traversal of the tree that holds no reference to it.
///
/// The queue holds the path to each node as the child indices from the root,
/// and the squared chord distance from the query to the node. The squared
/// chord increases with the great-circle distance, and the chord to an
/// envelope is never more than the chord to a leaf in it, so the leaves come
/// out nearest first.
#[derive(Clone, Debug)]
struct Cursor {
    query: UnitVec,
    origin: Position,
    filter: Filter,
    queue: BinaryHeap<Pending>,
}

impl Cursor {
    /// Starts a traversal from `origin`. Returns `None` if `origin` is not a
    /// valid position, for example because a coordinate is not finite.
    fn new(index: &VehicleIndex, origin: Position, filter: Filter) -> Option<Self> {
        let point = GeodeticPoint::try_new(origin.lon, origin.lat).ok()?;
        let mut cursor = Self {
            query: point.unit_vec(),
            origin,
            filter,
            queue: BinaryHeap::new(),
        };
        cursor.push_children(index.root(), &[]);
        Some(cursor)
    }

    fn next(&mut self, index: &VehicleIndex) -> Option<Candidate> {
        while let Some(pending) = self.queue.pop() {
            let node = node_at(index.root(), &pending.path).expect("the paths come from this tree");
            match node {
                RTreeNode::Leaf(leaf) => {
                    let vehicle = &index.vehicles[leaf.data];
                    if vehicle.range_m >= self.filter.min_range_m {
                        return Some(Candidate {
                            vehicle_id: vehicle.vehicle_id.clone(),
                            position: vehicle.position,
                            range_m: vehicle.range_m,
                            straight_line_m: self.origin.distance_m(vehicle.position),
                            ios_rental_uri: vehicle.ios_rental_uri.clone(),
                        });
                    }
                }
                RTreeNode::Parent(parent) => self.push_children(parent, &pending.path),
            }
        }
        None
    }

    fn push_children(&mut self, parent: &ParentNode<IndexedPoint>, path: &[usize]) {
        for (child_index, child) in parent.children().iter().enumerate() {
            let distance_2 = match child {
                RTreeNode::Leaf(leaf) => leaf.distance_2(&self.query),
                RTreeNode::Parent(node) => node.envelope().distance_2(&self.query),
            };
            let mut child_path = path.to_vec();
            child_path.push(child_index);
            self.queue.push(Pending {
                distance_2,
                path: child_path,
            });
        }
    }
}

/// Returns the node at `path` below `root`.
fn node_at<'a>(
    root: &'a ParentNode<IndexedPoint>,
    path: &[usize],
) -> Option<&'a RTreeNode<IndexedPoint>> {
    let (last, parents) = path.split_last()?;
    let mut parent = root;
    for &child_index in parents {
        match parent.children().get(child_index)? {
            RTreeNode::Parent(node) => parent = node,
            RTreeNode::Leaf(_) => return None,
        }
    }
    parent.children().get(*last)
}

/// A node in the queue of a [`Cursor`].
#[derive(Clone, Debug)]
struct Pending {
    distance_2: f64,
    path: Vec<usize>,
}

impl PartialEq for Pending {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Pending {}

impl PartialOrd for Pending {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pending {
    /// Orders by distance, reversed, so that `BinaryHeap` pops the nearest
    /// node first.
    fn cmp(&self, other: &Self) -> Ordering {
        other.distance_2.total_cmp(&self.distance_2)
    }
}
