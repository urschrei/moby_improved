import ActivityKit
import SwiftUI
import WidgetKit

@main
struct MyMobyWidgets: WidgetBundle {
  var body: some Widget {
    BikeLiveActivity()
  }
}

struct BikeLiveActivity: Widget {
  var body: some WidgetConfiguration {
    ActivityConfiguration(for: BikeActivityAttributes.self) { context in
      LockScreenView(state: context.state)
        .padding()
        .activityBackgroundTint(Color(.systemBackground).opacity(0.8))
    } dynamicIsland: { context in
      DynamicIsland {
        DynamicIslandExpandedRegion(.leading) {
          Label(context.state.headline, systemImage: "bicycle")
        }
        DynamicIslandExpandedRegion(.trailing) {
          if let rangeKm = context.state.rangeKm {
            Text("\(rangeKm) km")
          }
        }
        DynamicIslandExpandedRegion(.bottom) {
          if let message = context.state.message {
            Text(message).font(.footnote)
          }
        }
      } compactLeading: {
        Image(systemName: "bicycle")
      } compactTrailing: {
        Text(context.state.shortHeadline)
      } minimal: {
        Image(systemName: "bicycle")
      }
    }
  }
}

struct LockScreenView: View {
  let state: BikeActivityAttributes.ContentState

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      if let message = state.message {
        Label(message, systemImage: "exclamationmark.triangle.fill")
          .font(.subheadline.bold())
          .foregroundStyle(.orange)
      }
      HStack {
        Image(systemName: state.mode == .heading ? "figure.walk" : "bicycle")
          .font(.title2)
        Text(state.headline)
          .font(.title3.bold())
        Spacer()
        if let rangeKm = state.rangeKm {
          Label("\(rangeKm) km", systemImage: "battery.75percent")
            .foregroundStyle(.secondary)
        }
      }
      HStack {
        Text("\(state.bikeCount) bikes")
        Text("·")
        Text("updated \(state.updated, style: .relative) ago")
      }
      .font(.caption)
      .foregroundStyle(.secondary)
    }
  }
}

extension BikeActivityAttributes.ContentState {
  var headline: String {
    switch mode {
    case .heading:
      if let distanceM { "\(distanceM) m to your bike" } else { "Walking to your bike" }
    case .watching:
      if let walkMinutes { "Nearest bike: \(walkMinutes) min walk" } else { "No bike nearby" }
    }
  }

  var shortHeadline: String {
    switch mode {
    case .heading: distanceM.map { "\($0) m" } ?? "–"
    case .watching: walkMinutes.map { "\($0) min" } ?? "–"
    }
  }
}
