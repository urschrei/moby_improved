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
        .padding(16)
        .activityBackgroundTint(.black.opacity(0.7))
        .activitySystemActionForegroundColor(Theme.accent)
        .environment(\.colorScheme, .dark)
    } dynamicIsland: { context in
      DynamicIsland {
        DynamicIslandExpandedRegion(.leading) {
          BikeHeadline(state: context.state, size: 32)
        }
        DynamicIslandExpandedRegion(.trailing) {
          if let rangeKm = context.state.rangeKm {
            RangeSummary(rangeKm: rangeKm)
          }
        }
        DynamicIslandExpandedRegion(.bottom) {
          HStack {
            if let message = context.state.message {
              Text(message)
                .font(.footnote.weight(.semibold))
                .foregroundStyle(Theme.warning)
            }
            Spacer()
            WalkButton(link: context.state.walk)
          }
        }
      } compactLeading: {
        Image(systemName: context.state.mode == .watching ? "bicycle" : "figure.walk")
          .foregroundStyle(Theme.accent)
      } compactTrailing: {
        Text(context.state.shortHeadline)
          .font(.system(.body, design: .rounded).weight(.heavy))
          .foregroundStyle(Theme.accent)
      } minimal: {
        Image(systemName: "bicycle")
          .foregroundStyle(Theme.accent)
      }
    }
  }
}

struct LockScreenView: View {
  let state: BikeActivityAttributes.ContentState

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      if let message = state.message {
        Label(message, systemImage: "exclamationmark.triangle.fill")
          .font(.subheadline.weight(.semibold))
          .foregroundStyle(Theme.warning)
      }
      HStack(alignment: .center) {
        VStack(alignment: .leading, spacing: 2) {
          Text(state.title)
            .font(.subheadline.weight(.semibold))
            .foregroundStyle(Theme.accent)
          BikeHeadline(state: state, size: 44)
        }
        Spacer()
        WalkButton(link: state.walk)
      }
      HStack {
        if let rangeKm = state.rangeKm {
          RangeSummary(rangeKm: rangeKm)
        }
        Spacer()
        Text("Updated \(state.updated, style: .time)")
          .font(.caption)
          .foregroundStyle(.secondary)
      }
    }
  }
}

/// The walking time or the distance to the bike, as a large number.
struct BikeHeadline: View {
  let state: BikeActivityAttributes.ContentState
  let size: CGFloat

  var body: some View {
    if let (value, unit) = state.headlineParts {
      HStack(alignment: .firstTextBaseline, spacing: 4) {
        Text(value)
          .font(.system(size: size, weight: .heavy, design: .rounded))
          .monospacedDigit()
        Text(unit)
          .font(.headline)
          .foregroundStyle(.secondary)
      }
    } else {
      Text("No bike nearby")
        .font(.title3.weight(.semibold))
    }
  }
}

/// The range of the bike, with a gauge.
struct RangeSummary: View {
  let rangeKm: Int

  var body: some View {
    HStack(spacing: 6) {
      RangeGauge(rangeM: Double(rangeKm) * 1000)
      Text("\(rangeKm) km range")
        .font(.caption.weight(.medium))
        .monospacedDigit()
    }
  }
}

/// Opens the app, which starts walking to the bike in Maps.
struct WalkButton: View {
  let link: WalkLink?

  var body: some View {
    if let url = link?.url {
      Link(destination: url) {
        Label("Walk", systemImage: "figure.walk")
          .font(.subheadline.weight(.bold))
          .foregroundStyle(Theme.onAccent)
          .padding(.horizontal, 16)
          .padding(.vertical, 10)
          .background(Theme.accent, in: Capsule())
      }
    }
  }
}

extension BikeActivityAttributes.ContentState {
  var title: String {
    switch mode {
    case .watching: "Nearest bike"
    case .heading: "Walking to your bike"
    case .reserved: "Reserved in MOBY"
    }
  }

  /// The number and unit of the headline, or `nil` if there is no bike.
  var headlineParts: (String, String)? {
    switch mode {
    case .heading, .reserved:
      distanceM.map { ("\(($0 + 5) / 10 * 10)", "m to go") }
    case .watching:
      walkMinutes.map { ("\($0)", "min walk") }
    }
  }

  var shortHeadline: String {
    switch mode {
    case .heading, .reserved: distanceM.map { "\(($0 + 5) / 10 * 10) m" } ?? "–"
    case .watching: walkMinutes.map { "\($0) min" } ?? "–"
    }
  }
}
