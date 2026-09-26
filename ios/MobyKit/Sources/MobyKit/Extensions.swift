import CoreLocation

extension Bike: Identifiable {
  public var id: String { vehicleId }
}

extension WalkedBike: Identifiable {
  public var id: String { bike.vehicleId }
}

extension Coordinate {
  public var clLocation: CLLocationCoordinate2D {
    CLLocationCoordinate2D(latitude: lat, longitude: lon)
  }
}

extension Coordinate: Codable {
  private enum CodingKeys: String, CodingKey {
    case lat
    case lon
  }

  public init(from decoder: any Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    self.init(
      lat: try container.decode(Double.self, forKey: .lat),
      lon: try container.decode(Double.self, forKey: .lon))
  }

  public func encode(to encoder: any Encoder) throws {
    var container = encoder.container(keyedBy: CodingKeys.self)
    try container.encode(lat, forKey: .lat)
    try container.encode(lon, forKey: .lon)
  }
}

extension CommuteWindow: Codable, Identifiable {
  private enum CodingKeys: String, CodingKey {
    case weekdays
    case startMinute
    case endMinute
  }

  public var id: Self { self }

  public init(from decoder: any Decoder) throws {
    let container = try decoder.container(keyedBy: CodingKeys.self)
    self.init(
      weekdays: try container.decode(UInt8.self, forKey: .weekdays),
      startMinute: try container.decode(UInt16.self, forKey: .startMinute),
      endMinute: try container.decode(UInt16.self, forKey: .endMinute))
  }

  public func encode(to encoder: any Encoder) throws {
    var container = encoder.container(keyedBy: CodingKeys.self)
    try container.encode(weekdays, forKey: .weekdays)
    try container.encode(startMinute, forKey: .startMinute)
    try container.encode(endMinute, forKey: .endMinute)
  }
}

extension WalkedBike {
  /// The walking time, rounded up to whole minutes, and at least one minute.
  public var walkDuration: Duration {
    .seconds(walkMinutes * 60)
  }

  /// The walking time in whole minutes, rounded up, and at least 1.
  public var walkMinutes: Int {
    Int(max(1, (walkingS / 60).rounded(.up)))
  }
}
