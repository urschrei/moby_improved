import MapKit
import MobyKit
import UIKit

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

  /// Opens the MOBY app at the bike.
  ///
  /// The rental link is a Branch universal link. A browser that opens it
  /// goes to the App Store, so the link is first opened only as a universal
  /// link. iOS records a choice to open a domain in Safari for each domain,
  /// so the same path on the alternate Branch domain is tried next. The web
  /// link is the last choice.
  @MainActor
  static func openInMoby(_ bike: Bike) async {
    guard let url = rentalURL(for: bike) else { return }
    let application = UIApplication.shared
    for candidate in [url, alternateBranchURL(for: url)].compactMap(\.self) {
      if await application.open(candidate, options: [.universalLinksOnly: true]) {
        return
      }
      Log.routing.info("MOBY did not open \(candidate, privacy: .public) as a universal link")
    }
    await application.open(url)
  }

  /// Returns the URL on the alternate Branch domain, for example
  /// `moby-move-alternate.app.link` for `moby-move.app.link`.
  static func alternateBranchURL(for url: URL) -> URL? {
    guard var components = URLComponents(url: url, resolvingAgainstBaseURL: false),
      let host = components.host, host.hasSuffix(".app.link"), !host.contains("-alternate.")
    else {
      return nil
    }
    components.host = host.replacingOccurrences(of: ".app.link", with: "-alternate.app.link")
    return components.url
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
