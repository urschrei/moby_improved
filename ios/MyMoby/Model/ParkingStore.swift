import Foundation
import MobyKit
import Observation

/// The parking bays, from a copy of the `geofencing_zones` feed that is at
/// most a day old.
@MainActor
@Observable
final class ParkingStore {
  private(set) var parking: Parking?

  /// The e-bike type. Almost every MOBY bike in Dublin has this type.
  private static let eBikeTypeID = "3645"
  private static let maxAge: TimeInterval = 24 * 60 * 60

  private let client: GBFSClient

  init(client: GBFSClient) {
    self.client = client
  }

  /// Loads the bays from the cache, or from the feed if the cache is old.
  func load() async {
    guard parking == nil else { return }
    if let body = Self.cachedBody(),
      let parking = try? Parking.parse(body: body, vehicleTypeId: Self.eBikeTypeID)
    {
      self.parking = parking
      return
    }
    do {
      let body = try await client.geofencingZones()
      parking = try Parking.parse(body: body, vehicleTypeId: Self.eBikeTypeID)
      try? body.write(to: Self.cacheURL, options: .atomic)
      Log.refresh.info("parking has \(self.parking?.bayCount() ?? 0) bays")
    } catch {
      Log.refresh.error("parking failed: \(error.localizedDescription)")
    }
  }

  /// Returns the `count` bays nearest to `coordinate`.
  func nearest(to coordinate: Coordinate?, count: UInt32) -> [ParkingBay] {
    guard let coordinate, let parking else { return [] }
    return parking.nearest(coordinate: coordinate, k: count)
  }

  private static var cacheURL: URL {
    URL.cachesDirectory.appending(path: "geofencing_zones.json")
  }

  private static func cachedBody() -> Data? {
    guard
      let modified = try? cacheURL.resourceValues(forKeys: [.contentModificationDateKey])
        .contentModificationDate,
      modified.timeIntervalSinceNow > -maxAge
    else {
      return nil
    }
    return try? Data(contentsOf: cacheURL)
  }
}
