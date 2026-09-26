import MapKit
import MobyKit
import SwiftUI

struct SettingsView: View {
  @Bindable var settings: Settings
  let location: Coordinate?
  @Environment(\.dismiss) private var dismiss

  var body: some View {
    NavigationStack {
      Form {
        Section("Places") {
          ForEach(PlaceKind.allCases) { kind in
            NavigationLink {
              PlaceEditor(kind: kind, place: $settings.values.places[kind], location: location)
            } label: {
              LabeledContent {
                Text(settings.values.places[kind]?.name ?? "Not set")
              } label: {
                Label(kind.title, systemImage: kind.systemImage)
              }
            }
          }
        }
        Section {
          Stepper(value: $settings.values.minRangeKm, in: 0...50, step: 1) {
            LabeledContent("Minimum range", value: "\(Int(settings.values.minRangeKm)) km")
          }
        } header: {
          Text("Bikes")
        } footer: {
          Text("Bikes with less range are not shown.")
        }
        Section {
          ForEach($settings.values.windows) { $window in
            CommuteWindowEditor(window: $window)
          }
          .onDelete { settings.values.windows.remove(atOffsets: $0) }
          Button("Add commute time", systemImage: "plus") {
            settings.values.windows.append(
              CommuteWindow(weekdays: 0b001_1111, startMinute: 8 * 60, endMinute: 9 * 60))
          }
        } header: {
          Text("Commute times")
        } footer: {
          Text("The app refreshes more often at these times.")
        }
        Section("Refresh") {
          Stepper(value: $settings.values.commuteIntervalS, in: 15...120, step: 15) {
            LabeledContent("At commute times", value: "\(settings.values.commuteIntervalS) s")
          }
          Stepper(value: $settings.values.idleIntervalS, in: 30...300, step: 30) {
            LabeledContent("At other times", value: "\(settings.values.idleIntervalS) s")
          }
        }
      }
      .navigationTitle("Settings")
      .navigationBarTitleDisplayMode(.inline)
      .toolbar {
        ToolbarItem(placement: .confirmationAction) {
          Button("Done") { dismiss() }
        }
      }
    }
  }
}

/// Edits one commute window.
struct CommuteWindowEditor: View {
  @Binding var window: CommuteWindow

  private static let dayLetters = ["M", "T", "W", "T", "F", "S", "S"]
  private static let dayNames = Calendar.current.weekdaySymbols
  /// Calendar weekday symbols start on Sunday; the window bits start on Monday.
  private static func dayName(_ bit: Int) -> String { dayNames[(bit + 1) % 7] }

  var body: some View {
    VStack(alignment: .leading) {
      HStack {
        DatePicker(
          "Start", selection: minuteBinding(\.startMinute), displayedComponents: .hourAndMinute
        )
        .labelsHidden()
        Text("to")
        DatePicker(
          "End", selection: minuteBinding(\.endMinute), displayedComponents: .hourAndMinute
        )
        .labelsHidden()
      }
      HStack(spacing: 6) {
        ForEach(0..<7, id: \.self) { bit in
          let isOn = window.weekdays & (1 << bit) != 0
          Button(Self.dayLetters[bit]) {
            window.weekdays ^= 1 << bit
          }
          .buttonStyle(.bordered)
          .tint(isOn ? .accentColor : .secondary)
          .accessibilityLabel(Self.dayName(bit))
          .accessibilityAddTraits(isOn ? .isSelected : [])
        }
      }
      if window.startMinute >= window.endMinute {
        Text("The end must be after the start.")
          .font(.footnote)
          .foregroundStyle(.red)
      }
    }
  }

  private func minuteBinding(_ keyPath: WritableKeyPath<CommuteWindow, UInt16>) -> Binding<Date> {
    Binding(
      get: {
        Calendar.current.startOfDay(for: .now).addingTimeInterval(
          TimeInterval(window[keyPath: keyPath]) * 60)
      },
      set: { date in
        let parts = Calendar.current.dateComponents([.hour, .minute], from: date)
        window[keyPath: keyPath] = UInt16((parts.hour ?? 0) * 60 + (parts.minute ?? 0))
      }
    )
  }
}

/// Sets a saved place from the current location or an address search.
struct PlaceEditor: View {
  let kind: PlaceKind
  @Binding var place: Place?
  let location: Coordinate?
  @State private var query = ""
  @State private var results: [MKMapItem] = []
  @Environment(\.dismiss) private var dismiss

  /// Dublin, to bias the address search.
  private static let region = MKCoordinateRegion(
    center: CLLocationCoordinate2D(latitude: 53.3498, longitude: -6.2603),
    latitudinalMeters: 30_000, longitudinalMeters: 30_000)

  var body: some View {
    List {
      if let place {
        Section("Saved") {
          Text(place.name)
          Button("Remove", role: .destructive) {
            self.place = nil
          }
        }
      }
      Section {
        Button("Use current location", systemImage: "location") {
          Task { await useCurrentLocation() }
        }
        .disabled(location == nil)
      }
      Section("Search") {
        ForEach(results, id: \.self) { item in
          Button {
            place = Place(
              name: item.name ?? query,
              coordinate: Coordinate(
                lat: item.placemark.coordinate.latitude, lon: item.placemark.coordinate.longitude))
            dismiss()
          } label: {
            VStack(alignment: .leading) {
              Text(item.name ?? "")
              Text(item.placemark.title ?? "")
                .font(.footnote)
                .foregroundStyle(.secondary)
            }
          }
          .tint(.primary)
        }
      }
    }
    .navigationTitle(kind.title)
    .searchable(
      text: $query, placement: .navigationBarDrawer(displayMode: .always),
      prompt: "Address or place"
    )
    .onSubmit(of: .search) {
      Task { await search() }
    }
  }

  private func search() async {
    let request = MKLocalSearch.Request()
    request.naturalLanguageQuery = query
    request.region = Self.region
    results = (try? await MKLocalSearch(request: request).start().mapItems) ?? []
  }

  private func useCurrentLocation() async {
    guard let location else { return }
    let placemark = try? await CLGeocoder().reverseGeocodeLocation(
      CLLocation(latitude: location.lat, longitude: location.lon)
    ).first
    place = Place(name: placemark?.name ?? "Current location", coordinate: location)
    dismiss()
  }
}
