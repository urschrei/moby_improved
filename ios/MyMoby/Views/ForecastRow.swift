import MobyKit
import SwiftUI

/// The chance of a bike within 300 m walk of a saved place at the start of a
/// commute window, and the bikes there now.
struct ForecastRow: View {
  let title: String
  let aim: ForecastAim
  let state: ForecastStore.State
  let isRequesting: Bool
  let retry: () -> Void

  /// Below this percentage, a bike is unlikely.
  static let unlikelyBelow: UInt8 = 40

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      HStack(alignment: .firstTextBaseline) {
        Text(title)
          .font(.subheadline.weight(.semibold))
        Spacer()
        Text("\(Self.clock(aim.startMs))–\(Self.clock(aim.endMs))")
          .font(.footnote)
          .foregroundStyle(.secondary)
          .monospacedDigit()
          .accessibilityLabel("Commute from \(Self.clock(aim.startMs)) to \(Self.clock(aim.endMs))")
      }
      .accessibilityElement(children: .combine)
      .accessibilityAddTraits(.isHeader)
      content
    }
    .frame(maxWidth: .infinity, alignment: .leading)
    .animation(.snappy, value: state)
  }

  @ViewBuilder private var content: some View {
    switch state {
    case .none:
      EmptyView()
    case .loading:
      HStack(spacing: 8) {
        ProgressView()
        Text("Loading the forecast")
      }
      .font(.subheadline)
      .foregroundStyle(.secondary)
    case .loaded(let forecast):
      if let percent = forecast.percent {
        Chance(percent: percent, at: Self.clock(aim.atMs))
      } else {
        Text("No forecast for \(Self.clock(aim.atMs))")
          .font(.subheadline)
          .foregroundStyle(.secondary)
      }
      BikeCount(certain: forecast.certain, suspect: forecast.suspect, at: nil)
    case .paused(let pause):
      switch pause.reason {
      case .feedTooOld:
        // The orange of the warning does not have sufficient contrast for
        // text on white, so only the symbol has it.
        Label {
          Text("MOBY’s bike data stopped at \(Self.clock(pause.originMs)). Forecast paused.")
        } icon: {
          Image(systemName: "exclamationmark.triangle.fill")
            .foregroundStyle(Theme.warning)
        }
        .font(.subheadline.weight(.semibold))
        BikeCount(
          certain: pause.certain, suspect: pause.suspect, at: Self.clock(pause.originMs))
      case .parametersTooOld:
        Label("Forecast paused until the nightly update runs.", systemImage: "info.circle")
          .font(.subheadline)
          .foregroundStyle(.secondary)
        BikeCount(certain: pause.certain, suspect: pause.suspect, at: nil)
      }
    case .pending:
      Text("The forecast for this place starts after the nightly update.")
        .font(.subheadline)
        .foregroundStyle(.secondary)
    case .failed(let isOffline, let last):
      VStack(alignment: .leading, spacing: 8) {
        Text(Self.failure(isOffline: isOffline, last: last))
          .font(.subheadline)
          .foregroundStyle(.secondary)
        Button(action: retry) {
          HStack(spacing: 6) {
            if isRequesting {
              ProgressView().controlSize(.small)
            }
            Text("Try Again")
          }
        }
        .buttonStyle(.bordered)
        .font(.subheadline.weight(.semibold))
        .disabled(isRequesting)
      }
    }
  }

  private static func failure(isOffline: Bool, last: PlaceForecast?) -> String {
    let reason = isOffline ? "Can’t reach the forecast service." : "No forecast now."
    guard let last, let percent = last.percent else { return reason }
    return "\(reason) At \(clock(last.issuedMs)), the forecast was \(percent)%."
  }

  static func clock(_ milliseconds: Int64) -> String {
    Date(timeIntervalSince1970: Double(milliseconds) / 1000)
      .formatted(date: .omitted, time: .shortened)
  }
}

/// A percentage chance of a bike, above a gauge.
private struct Chance: View {
  let percent: UInt8
  let at: String

  var body: some View {
    let colour = percent < ForecastRow.unlikelyBelow ? Theme.caution : Theme.accent
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text("\(percent)%")
          .font(.system(size: 34, weight: .heavy, design: .rounded))
          .monospacedDigit()
          .foregroundStyle(colour)
          .contentTransition(.numericText())
        Text("chance of a bike at \(at)")
          .font(.subheadline)
          .foregroundStyle(.secondary)
      }
      Gauge(value: Double(percent), in: 0...100) {}
        .gaugeStyle(.linearCapacity)
        .tint(colour)
        .accessibilityHidden(true)
    }
    .accessibilityElement(children: .combine)
  }
}

/// The bikes within 300 m walk, and those that may still be on a ride.
private struct BikeCount: View {
  let certain: UInt32
  let suspect: UInt32
  /// The time of the count, or `nil` for now.
  let at: String?

  var body: some View {
    VStack(alignment: .leading, spacing: 2) {
      Label {
        if let at {
          Text("\(Self.bikes(certain)) within a 300 m walk at \(at)")
            .foregroundStyle(.secondary)
        } else {
          Text("\(Text(Self.bikes(certain)).fontWeight(.semibold)) within a 300 m walk now")
        }
      } icon: {
        Image(systemName: "bicycle")
          .foregroundStyle(.secondary)
      }
      .font(.subheadline)
      if suspect > 0 {
        Text(
          certain == 0
            ? "\(Self.bikes(suspect)) may still be on a ride"
            : "\(suspect) more may still be on a ride"
        )
        .font(.footnote)
        .foregroundStyle(.secondary)
      }
    }
    .accessibilityElement(children: .combine)
  }

  private static func bikes(_ count: UInt32) -> String {
    switch count {
    case 0: "No bikes"
    case 1: "1 bike"
    default: "\(count) bikes"
    }
  }
}
