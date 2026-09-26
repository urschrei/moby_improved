import Foundation
import MobyKit

/// Gives the current `vehicle_status` feed.
protocol FeedSource: Sendable {
  func vehicleStatus() async throws -> Feed
}

#if DEBUG
  /// Replays two feed responses captured three minutes apart, to test the
  /// response to a bike that another rider rents. Enable it with the
  /// `-replay` launch argument.
  actor ReplaySource: FeedSource {
    private var requests = 0

    func vehicleStatus() async throws -> Feed {
      let name = requests == 0 ? "vehicle_status_1118" : "vehicle_status_1121"
      requests += 1
      guard
        let url = Bundle.main.url(forResource: name, withExtension: "json", subdirectory: "probe")
      else {
        throw CocoaError(.fileNoSuchFile)
      }
      return try Feed.parse(body: Data(contentsOf: url))
    }
  }
#endif
