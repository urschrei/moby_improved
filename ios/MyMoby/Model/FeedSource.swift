import Foundation
import MobyKit

/// Gives the current `vehicle_status` feed.
protocol FeedSource: Sendable {
  func vehicleStatus() async throws -> Feed
}

/// Gives the feed captured in the Rust test fixtures.
struct FixtureSource: FeedSource {
  func vehicleStatus() async throws -> Feed {
    guard let url = Bundle.main.url(forResource: "vehicle_status", withExtension: "json") else {
      throw CocoaError(.fileNoSuchFile)
    }
    return try Feed.parse(body: Data(contentsOf: url))
  }
}
