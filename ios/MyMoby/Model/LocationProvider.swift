import CoreLocation
import MobyKit
import Observation

/// The current position of the device.
@MainActor
@Observable
final class LocationProvider {
  private(set) var coordinate: Coordinate?
  private(set) var isDenied = false

  /// Receives location updates until the task is cancelled.
  func run() async {
    let session = CLServiceSession(authorization: .whenInUse)
    defer { session.invalidate() }
    do {
      for try await update in CLLocationUpdate.liveUpdates(.otherNavigation) {
        isDenied = update.authorizationDenied || update.authorizationDeniedGlobally
        if let location = update.location {
          coordinate = Coordinate(
            lat: location.coordinate.latitude, lon: location.coordinate.longitude)
        }
      }
    } catch {
      // The stream ends only when the task is cancelled.
    }
  }
}
