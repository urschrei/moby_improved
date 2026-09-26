import ActivityKit
import Foundation

/// The Live Activity that shows the nearest bike, or the bike the rider is
/// walking to.
struct BikeActivityAttributes: ActivityAttributes {
  struct ContentState: Codable, Hashable {
    enum Mode: String, Codable, Hashable {
      /// The activity shows the nearest bike.
      case watching
      /// The activity shows the bike the rider is walking to.
      case heading
    }

    var mode: Mode
    /// The walking time to the bike, in minutes.
    var walkMinutes: Int?
    /// The distance to the bike, in metres.
    var distanceM: Int?
    /// The range of the bike, in kilometres.
    var rangeKm: Int?
    /// The number of bikes with enough range.
    var bikeCount: Int
    /// A message about a change, such as a rented bike.
    var message: String?
    /// The time of the feed.
    var updated: Date
  }
}
