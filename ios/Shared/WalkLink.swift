import Foundation

/// A link that opens the app and starts walking to a bike. The Live Activity
/// uses it, because a link in a Live Activity always opens its own app.
struct WalkLink: Codable, Hashable {
  var vehicleID: String
  var latitude: Double
  var longitude: Double

  static let scheme = "mymoby"
  private static let host = "walk"

  var url: URL? {
    var components = URLComponents()
    components.scheme = Self.scheme
    components.host = Self.host
    components.queryItems = [
      URLQueryItem(name: "id", value: vehicleID),
      URLQueryItem(name: "lat", value: String(latitude)),
      URLQueryItem(name: "lon", value: String(longitude)),
    ]
    return components.url
  }

  /// Reads a link from `url`. Returns `nil` if `url` is not a walk link.
  init?(url: URL) {
    guard url.scheme == Self.scheme, url.host == Self.host,
      let items = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems,
      let vehicleID = items.first(where: { $0.name == "id" })?.value,
      let latitude = items.first(where: { $0.name == "lat" })?.value.flatMap(Double.init),
      let longitude = items.first(where: { $0.name == "lon" })?.value.flatMap(Double.init)
    else {
      return nil
    }
    self.init(vehicleID: vehicleID, latitude: latitude, longitude: longitude)
  }

  init(vehicleID: String, latitude: Double, longitude: Double) {
    self.vehicleID = vehicleID
    self.latitude = latitude
    self.longitude = longitude
  }
}
