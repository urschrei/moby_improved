use geo::Centroid as _;
use geo::Coord;
use geo::LineString;
use geo::Orient as _;
use geo::Polygon;
use geo::orient::Direction;
use rstar::primitives::GeomWithData;
use rstar_geodetic::GeodeticCoord;
use rstar_geodetic::GeodeticPolygon;
use rstar_geodetic::GeodeticRTree;

use crate::Position;
use crate::gbfs::GeofencingZones;
use crate::gbfs::ZoneHash;

/// The zones where a rider can end a ride, indexed for nearest-neighbour
/// queries on the sphere.
///
/// MOBY publishes its parking bays as small zones in which a ride can end;
/// outside them, the global rules do not allow it.
#[derive(Debug)]
pub struct ParkingIndex {
    tree: GeodeticRTree<GeomWithData<GeodeticPolygon, usize>>,
    bays: Vec<Bay>,
}

/// A zone where a rider can end a ride, or one polygon of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Bay {
    /// The hash of the zone. The polygons of one zone have the same hash.
    pub zone_hash: ZoneHash,
    /// The centroid of the zone.
    pub centroid: Position,
    /// The exterior ring of the zone, counter-clockwise.
    pub outline: Vec<Position>,
}

/// A bay and its distance from a query position.
#[derive(Clone, Debug, PartialEq)]
pub struct NearbyBay {
    /// The bay.
    pub bay: Bay,
    /// The great-circle distance in metres from the query position to the
    /// nearest point of the bay. It is zero inside the bay.
    pub distance_m: f64,
}

impl ParkingIndex {
    /// Indexes the zones in which a vehicle of type `vehicle_type_id` can end
    /// a ride.
    ///
    /// A zone qualifies if the first of its rules that applies to the type
    /// allows the ride to end. Polygons that `rstar_geodetic` rejects, for
    /// example because a ring has fewer than three distinct points, are
    /// skipped.
    #[must_use]
    pub fn new(zones: &GeofencingZones, vehicle_type_id: &str) -> Self {
        let polygons: Vec<(&ZoneHash, Polygon)> = zones
            .geofencing_zones
            .features
            .iter()
            .filter(|zone| {
                zone.properties
                    .rule_for(vehicle_type_id)
                    .is_some_and(|rule| rule.ride_end_allowed)
            })
            .flat_map(|zone| {
                zone.geometry
                    .coordinates
                    .iter()
                    .map(|rings| (&zone.hash, rings))
            })
            .filter_map(|(hash, rings)| Some((hash, polygon(rings)?)))
            .collect();

        let mut bays = Vec::new();
        let mut leaves = Vec::new();
        for (zone_hash, polygon) in polygons {
            let Some(centroid) = polygon.centroid() else {
                continue;
            };
            let Ok(geodetic) = GeodeticPolygon::try_from(polygon.clone()) else {
                continue;
            };
            leaves.push(GeomWithData::new(geodetic, bays.len()));
            bays.push(Bay {
                zone_hash: zone_hash.clone(),
                centroid: Position::new(centroid.y(), centroid.x()),
                outline: polygon
                    .exterior()
                    .coords()
                    .map(|coord| Position::new(coord.y, coord.x))
                    .collect(),
            });
        }
        Self {
            tree: GeodeticRTree::bulk_load(leaves),
            bays,
        }
    }

    /// Returns the `k` bays nearest to `position`, nearest first.
    #[must_use]
    pub fn nearest(&self, position: Position, k: usize) -> Vec<NearbyBay> {
        self.tree
            .nearest_neighbor_iter_with_distance(GeodeticCoord {
                lon: position.lon,
                lat: position.lat,
            })
            .take(k)
            .map(|(leaf, distance_m)| NearbyBay {
                bay: self.bays[leaf.data].clone(),
                distance_m,
            })
            .collect()
    }

    /// Returns the number of bays in the index.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bays.len()
    }

    /// Returns `true` if the index has no bays.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bays.is_empty()
    }
}

/// Makes a polygon from GeoJSON rings, with the exterior ring
/// counter-clockwise and holes clockwise, as `rstar_geodetic` requires.
/// MOBY publishes its exterior rings clockwise.
fn polygon(rings: &[Vec<[f64; 2]>]) -> Option<Polygon> {
    let (exterior, interiors) = rings.split_first()?;
    let ring = |points: &Vec<[f64; 2]>| {
        LineString::new(points.iter().map(|&[x, y]| Coord { x, y }).collect())
    };
    Some(
        Polygon::new(ring(exterior), interiors.iter().map(ring).collect())
            .orient(Direction::Default),
    )
}
