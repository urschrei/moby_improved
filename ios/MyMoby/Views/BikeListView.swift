import MobyKit
import SwiftUI

struct BikeListView: View {
  let store: BikeStore

  var body: some View {
    NavigationStack {
      List {
        if let error = store.lastError {
          Text(error).foregroundStyle(.red)
        }
        ForEach(store.bikes.prefix(20), id: \.vehicleId) { bike in
          BikeRow(bike: bike)
        }
      }
      .navigationTitle("Nearby bikes")
      .refreshable { await store.refresh() }
      .task { await store.refresh() }
    }
  }
}

struct BikeRow: View {
  let bike: Bike

  var body: some View {
    HStack {
      Text(
        Measurement(value: bike.straightLineM, unit: UnitLength.meters),
        format: .measurement(width: .abbreviated, usage: .road))
      Spacer()
      Label(
        Measurement(value: bike.rangeM / 1000, unit: UnitLength.kilometers)
          .formatted(
            .measurement(
              width: .abbreviated, numberFormatStyle: .number.precision(.fractionLength(0)))),
        systemImage: "battery.75percent"
      )
      .foregroundStyle(.secondary)
    }
    .accessibilityElement(children: .combine)
  }
}
