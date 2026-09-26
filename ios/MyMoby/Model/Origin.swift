import MobyKit

/// A place that the user can save.
enum PlaceKind: String, CaseIterable, Codable, Identifiable, CodingKeyRepresentable {
  case home
  case creche
  case work
  case secondWork

  var id: Self { self }

  var title: String {
    switch self {
    case .home: "Home"
    case .creche: "Crèche"
    case .work: "Work"
    case .secondWork: "Work 2"
    }
  }

  var systemImage: String {
    switch self {
    case .home: "house"
    case .creche: "figure.and.child.holdinghands"
    case .work: "briefcase"
    case .secondWork: "building.2"
    }
  }
}

/// The place to search for bikes from.
enum Origin: Hashable, Identifiable {
  case here
  case place(PlaceKind)

  static let allCases: [Origin] = [.here] + PlaceKind.allCases.map(Origin.place)

  var id: Self { self }

  var title: String {
    switch self {
    case .here: "Here"
    case .place(let kind): kind.title
    }
  }

  /// Returns the coordinate of the origin, if it is known.
  @MainActor
  func coordinate(location: Coordinate?, settings: Settings) -> Coordinate? {
    switch self {
    case .here: location
    case .place(let kind): settings.values.places[kind]?.coordinate
    }
  }

  /// Returns the saved place that is nearest to `location`, or `nil` if no
  /// place is saved or the location is not known.
  @MainActor
  static func nearestPlace(to location: Coordinate?, settings: Settings) -> Origin? {
    guard let location else {
      return nil
    }
    return settings.values.places
      .min {
        distanceM(a: location, b: $0.value.coordinate)
          < distanceM(a: location, b: $1.value.coordinate)
      }
      .map { .place($0.key) }
  }
}
