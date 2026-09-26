import MobyKit
import SwiftUI

/// The age of the feed and the number of bikes, updated every second.
struct FeedStatus: View {
  let feed: Feed?
  let bikeCount: Int
  let isRefreshing: Bool

  /// The age after which the feed counts as stale.
  static let staleAfterS: Int64 = 600

  var body: some View {
    TimelineView(.periodic(from: .now, by: 1)) { context in
      HStack(spacing: 6) {
        if let feed {
          let age = feed.age(nowMs: context.date.milliseconds, staleAfterS: Self.staleAfterS)
          if age.isStale {
            Image(systemName: "exclamationmark.triangle.fill")
              .foregroundStyle(.orange)
            Text(
              "Data is \(Duration.seconds(age.ageS), format: .units(allowed: [.hours, .minutes], width: .wide)) old"
            )
          } else {
            Text(
              "Updated \(Duration.seconds(age.ageS), format: .units(allowed: [.minutes, .seconds], width: .abbreviated)) ago"
            )
          }
          Text("·")
          Text("\(bikeCount) bikes")
        } else {
          Text("Loading bikes…")
        }
        Spacer()
        if isRefreshing {
          ProgressView().controlSize(.mini)
        }
      }
      .font(.footnote)
      .foregroundStyle(.secondary)
      .monospacedDigit()
      .padding(.horizontal)
      .padding(.vertical, 6)
      .accessibilityElement(children: .combine)
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
