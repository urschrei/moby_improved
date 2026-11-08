import MobyKit
import SwiftUI

/// The bikes within 300 m walk of a saved place now, and the probability of
/// a bike there later.
struct ForecastSection: View {
  let title: String
  let state: ForecastStore.State

  var body: some View {
    switch state {
    case .none:
      EmptyView()
    case .loading:
      section {
        Text("Loading the forecast")
          .foregroundStyle(.secondary)
      }
    case .pending:
      section {
        Text("The forecast for this place starts after the next nightly fit.")
          .foregroundStyle(.secondary)
      }
    case .unavailable:
      section {
        Text("No forecast now")
          .foregroundStyle(.secondary)
      }
    case .loaded(let forecast):
      section {
        Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 6) {
          GridRow {
            Text("Now")
            Text(Self.count(forecast.certain))
              .monospacedDigit()
            if forecast.suspect > 0 {
              Text("\(forecast.suspect) more not confirmed")
                .foregroundStyle(.secondary)
            }
          }
          .accessibilityElement(children: .combine)
          ForEach(forecast.targets, id: \.aheadMin) { target in
            if let percent = target.percent {
              GridRow {
                Text(Self.ahead(minutes: target.aheadMin))
                Text("\(percent)%")
                  .monospacedDigit()
                Text(Self.clock(milliseconds: target.atMs))
                  .foregroundStyle(.secondary)
              }
              .accessibilityElement(children: .ignore)
              .accessibilityLabel(
                "At \(Self.clock(milliseconds: target.atMs)), \(percent)% chance of a bike")
            }
          }
        }
        .font(.subheadline)
      }
    }
  }

  private func section(@ViewBuilder content: () -> some View) -> some View {
    VStack(alignment: .leading, spacing: 8) {
      Text("Bikes within 300 m of \(title)")
        .font(.subheadline.weight(.semibold))
        .accessibilityAddTraits(.isHeader)
      content()
        .font(.subheadline)
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }

  private static func count(_ bikes: UInt32) -> String {
    bikes == 1 ? "1 bike" : "\(bikes) bikes"
  }

  private static func ahead(minutes: Int64) -> String {
    minutes < 60 ? "+\(minutes) min" : "+\(minutes / 60) h"
  }

  private static func clock(milliseconds: Int64) -> String {
    Date(timeIntervalSince1970: Double(milliseconds) / 1000)
      .formatted(date: .omitted, time: .shortened)
  }
}
