import CoreLocation
import MobyKit
import Observation

/// The streets of bikes, from reverse geocoding. The app looks up only the
/// bike that the sheet shows, because the geocoder limits the request rate.
@MainActor
@Observable
final class StreetNames {
  private var entries: [String: Entry] = [:]
  @ObservationIgnored private let geocoder = CLGeocoder()

  /// Returns the street of `bike`, if it is known for the bike's position.
  func street(for bike: Bike) -> String? {
    entries[bike.id].flatMap { $0.coordinate == bike.coordinate ? $0.street : nil }
  }

  /// Looks up the street of `bike`, if it is not known.
  func lookUp(_ bike: Bike) async {
    guard street(for: bike) == nil, !geocoder.isGeocoding else { return }
    let location = CLLocation(latitude: bike.coordinate.lat, longitude: bike.coordinate.lon)
    guard let placemark = try? await geocoder.reverseGeocodeLocation(location).first,
      let street = placemark.thoroughfare
    else {
      return
    }
    entries[bike.id] = Entry(coordinate: bike.coordinate, street: street)
  }

  private struct Entry {
    let coordinate: Coordinate
    let street: String
  }
}
