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
