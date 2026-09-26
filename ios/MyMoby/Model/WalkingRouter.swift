import MapKit
import MobyKit

/// Ranks bikes by walking distance with MapKit routes.
///
/// The Rust `Walker` decides which bikes to route. This type does the
/// requests, with at most `k` in flight at a time.
@MainActor
final class WalkingRouter {
  private let walker = Walker()

  func rank(origin: Coordinate, bikes: [Bike], k: UInt32, maxRequests: UInt32) async
    -> [WalkedBike]
  {
    walker.start(origin: origin, bikes: bikes, k: k, maxRequests: maxRequests)
    await withTaskGroup(of: (String, Walk?).self) { group in
      while let bike = walker.nextRequest() {
        group.addTask { (bike.vehicleId, await Self.eta(from: origin, to: bike)) }
      }
      for await (vehicleId, eta) in group {
        if let eta {
          walker.reportRoute(vehicleId: vehicleId, walkingM: eta.metres, walkingS: eta.seconds)
        } else {
          walker.reportFailure(vehicleId: vehicleId)
        }
        while let bike = walker.nextRequest() {
          group.addTask { (bike.vehicleId, await Self.eta(from: origin, to: bike)) }
        }
      }
    }
    let results = walker.results()
    walker.finish()
    return results
  }

  /// The length and duration of a walking route.
  private struct Walk: Sendable {
    let metres: Double
    let seconds: Double
  }

  private nonisolated static func eta(from origin: Coordinate, to bike: Bike) async -> Walk? {
    let request = MKDirections.Request()
    request.source = MKMapItem(placemark: MKPlacemark(coordinate: origin.clLocation))
    request.destination = MKMapItem(placemark: MKPlacemark(coordinate: bike.coordinate.clLocation))
    request.transportType = .walking
    guard let eta = try? await MKDirections(request: request).calculateETA() else {
      return nil
    }
    return Walk(metres: eta.distance, seconds: eta.expectedTravelTime)
  }
}
