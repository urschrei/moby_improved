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

/// The time until the next attempt after a failed refresh, updated every
/// second.
struct RetryCountdown: View {
  let nextRefresh: Date?
  let isRefreshing: Bool

  var body: some View {
    TimelineView(.periodic(from: .now, by: 1)) { context in
      if isRefreshing {
        Text("Trying again now.")
      } else if let nextRefresh, nextRefresh > context.date {
        let seconds = Int(nextRefresh.timeIntervalSince(context.date).rounded(.up))
        Text("Trying again in \(seconds) s.")
          .monospacedDigit()
      }
    }
  }
}

/// A warning that the last refresh failed, above bikes from an earlier one.
struct RefreshFailure: View {
  let message: String
  let nextRefresh: Date?
  let isRefreshing: Bool

  var body: some View {
    Label {
      HStack(spacing: 4) {
        Text(message)
        RetryCountdown(nextRefresh: nextRefresh, isRefreshing: isRefreshing)
      }
    } icon: {
      Image(systemName: "wifi.exclamationmark")
    }
    .font(.footnote.weight(.semibold))
    .foregroundStyle(Theme.warning)
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
