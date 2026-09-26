import CoreLocation
import MobyKit
import Observation

/// The current position of the device.
///
/// The provider uses `CLLocationManager` with automatic pausing off. While
/// background updates are on, iOS keeps the app running even if the device
/// does not move, for example while the rider waits at home.
@MainActor
@Observable
final class LocationProvider: NSObject, CLLocationManagerDelegate {
  private(set) var coordinate: Coordinate?
  /// The horizontal accuracy of `coordinate`, in metres.
  private(set) var accuracyM: Double?
  private(set) var isDenied = false

  @ObservationIgnored private let manager = CLLocationManager()

  override init() {
    super.init()
    manager.delegate = self
    manager.activityType = .fitness
    manager.desiredAccuracy = kCLLocationAccuracyNearestTenMeters
    manager.distanceFilter = 10
    manager.pausesLocationUpdatesAutomatically = false
  }

  /// Asks for permission and starts location updates.
  func start() {
    manager.requestWhenInUseAuthorization()
    manager.startUpdatingLocation()
  }

  /// Turns background location updates on or off. Turn them on only while
  /// the app is in the foreground.
  func setBackgroundUpdates(_ isEnabled: Bool) {
    manager.allowsBackgroundLocationUpdates = isEnabled
    manager.showsBackgroundLocationIndicator = isEnabled
  }

  nonisolated func locationManager(
    _ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]
  ) {
    guard let location = locations.last else { return }
    let coordinate = Coordinate(
      lat: location.coordinate.latitude, lon: location.coordinate.longitude)
    // The manager calls its delegate on the main thread, because it was
    // made on the main thread.
    let accuracyM = location.horizontalAccuracy
    MainActor.assumeIsolated {
      self.coordinate = coordinate
      self.accuracyM = accuracyM
    }
  }

  nonisolated func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
    let status = manager.authorizationStatus
    MainActor.assumeIsolated {
      isDenied = status == .denied || status == .restricted
    }
  }
}
