import Foundation
import MobyKit
import Observation

/// The bikes near the current origin, and the state of the last refresh.
@MainActor
@Observable
final class BikeStore {
  private(set) var bikes: [Bike] = []
  private(set) var feed: Feed?
  private(set) var lastError: String?
  private(set) var isRefreshing = false

  var origin = Coordinate(lat: 53.349805, lon: -6.26031)
  var minRangeM: Double = 10_000

  private let source: any FeedSource

  init(source: any FeedSource) {
    self.source = source
  }

  func refresh() async {
    isRefreshing = true
    defer { isRefreshing = false }
    do {
      let feed = try await source.vehicleStatus()
      self.feed = feed
      bikes = feed.bikes(origin: origin, minRangeM: minRangeM)
      lastError = nil
    } catch {
      lastError = error.localizedDescription
    }
  }
}
