import MobyKit
import SwiftUI

/// The age of the feed and the number of bikes, updated every second.
struct FeedStatus: View {
  let feed: Feed?
  let bikeCount: Int
  let minRangeKm: Double
  let isRefreshing: Bool

  /// The age after which the feed counts as stale.
  static let staleAfterS: Int64 = 600

  var body: some View {
    TimelineView(.periodic(from: .now, by: 1)) { context in
      HStack(spacing: 6) {
        if let feed {
          let age = feed.age(nowMs: context.date.milliseconds, staleAfterS: Self.staleAfterS)
          Text(
            "Updated \(Duration.seconds(age.ageS), format: .units(allowed: [.hours, .minutes, .seconds], width: .narrow, maximumUnitCount: 1)) ago"
          )
          .foregroundStyle(age.isStale ? Theme.warning : .secondary)
          if isRefreshing {
            ProgressView().controlSize(.mini)
          }
          Spacer()
          Text("\(bikeCount) bikes over \(Int(minRangeKm)) km")
        } else {
          Text("Loading bikes…")
          Spacer()
        }
      }
      .font(.footnote)
      .foregroundStyle(.secondary)
      .monospacedDigit()
      .accessibilityElement(children: .combine)
    }
  }
}

/// A warning that the feed is old.
struct StaleNotice: View {
  let feed: Feed?

  var body: some View {
    TimelineView(.periodic(from: .now, by: 10)) { context in
      if let feed {
        let age = feed.age(nowMs: context.date.milliseconds, staleAfterS: FeedStatus.staleAfterS)
        if age.isStale {
          Label(
            "The bike data is \(Duration.seconds(age.ageS), format: .units(allowed: [.hours, .minutes], width: .wide, maximumUnitCount: 1)) old. Bikes may have gone.",
            systemImage: "exclamationmark.triangle.fill"
          )
          .font(.subheadline.weight(.semibold))
          .foregroundStyle(Theme.warning)
          .padding(.bottom, 16)
        }
      }
    }
  }
}

/// The CC BY attribution for the MOBY data.
struct Attribution: View {
  var body: some View {
    Text(
      "Bike data: [Moby Bikes API](https://data.smartdublin.ie/dataset/moby-bikes), Dublin City Council / Smart Dublin, [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)."
    )
    .font(.caption2)
    .foregroundStyle(.secondary)
  }
}
