import MapKit
import MobyKit

/// Hands a bike over to Apple Maps or to the MOBY app.
enum Handoff {
  /// Opens Apple Maps with walking directions to the bike.
  @MainActor
  static func walk(to bike: Bike) {
    walk(to: bike.coordinate)
  }

  /// Opens Apple Maps with walking directions to a position.
  @MainActor
  static func walk(to coordinate: Coordinate) {
    let item = MKMapItem(placemark: MKPlacemark(coordinate: coordinate.clLocation))
    item.name = "MOBY bike"
    item.openInMaps(launchOptions: [
      MKLaunchOptionsDirectionsModeKey: MKLaunchOptionsDirectionsModeWalking
    ])
  }

  /// The link that opens the MOBY app at the bike.
  static func rentalURL(for bike: Bike) -> URL? {
    bike.rentalUri.flatMap(URL.init(string:))
  }

  /// Returns the walking route to the bike, for display on the map.
  static func route(from origin: Coordinate, to bike: Bike) async -> MKRoute? {
    let request = MKDirections.Request()
    request.source = MKMapItem(placemark: MKPlacemark(coordinate: origin.clLocation))
    request.destination = MKMapItem(placemark: MKPlacemark(coordinate: bike.coordinate.clLocation))
    request.transportType = .walking
    return try? await MKDirections(request: request).calculate().routes.first
  }
}
