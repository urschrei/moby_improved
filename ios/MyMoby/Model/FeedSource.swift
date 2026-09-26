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

  /// Times out on the first `failures` requests, then fetches the live feed,
  /// to test the retries after a failed refresh. Enable it with the
  /// `-fail-first N` launch arguments.
  actor FailingSource: FeedSource {
    private var failures: Int
    private let source: any FeedSource

    init(failures: Int, source: any FeedSource) {
      self.failures = failures
      self.source = source
    }

    func vehicleStatus() async throws -> Feed {
      if failures > 0 {
        failures -= 1
        throw URLError(.timedOut)
      }
      return try await source.vehicleStatus()
    }
  }
#endif
