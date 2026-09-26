import MapKit
import MobyKit
import SwiftUI

/// The map of nearby bikes above the list of the nearest bikes on foot.
struct NearbyView: View {
  let controller: CommuteController
  let location: LocationProvider
  let settings: Settings
  let parking: ParkingStore
  @Environment(\.scenePhase) private var scenePhase
  @State private var origin: Origin = .here
  @State private var isShowingSettings = false
  @State private var camera: MapCameraPosition = .userLocation(fallback: .automatic)
  @State private var selectedID: String?
  @State private var route: MKRoute?

  /// The number of bikes to show on the map.
  private let mapLimit = 60

  private var store: BikeStore { controller.store }

  var body: some View {
    NavigationStack {
      VStack(spacing: 0) {
        OriginPicker(origin: $origin, settings: settings, location: location.coordinate)
        BikeMap(
          bikes: Array(store.bikes.prefix(mapLimit)),
          ranked: store.nearest,
          featuredID: store.nearest.first?.id,
          target: store.target,
          bays: bays,
          route: route,
          camera: $camera,
          selectedID: $selectedID
        )
        .frame(maxHeight: .infinity)
        FeedStatus(feed: store.feed, bikeCount: store.bikes.count, isRefreshing: store.isRefreshing)
        if let target = store.target {
          TargetBanner(target: target, origin: location.coordinate, controller: controller)
        }
        BikeList(
          store: store, location: location, origin: originCoordinate, nearestBay: bays.first,
          selectedID: $selectedID
        )
        .frame(maxHeight: .infinity)
      }
      .navigationTitle("Nearby bikes")
      .navigationBarTitleDisplayMode(.inline)
      .toolbar {
        ToolbarItem(placement: .topBarLeading) {
          if controller.watch.isRunning {
            Button("Stop watching", systemImage: "eye.slash") {
              Task { await controller.stopWatching() }
            }
          } else {
            Button("Watch in the background", systemImage: "eye") {
              controller.startWatching()
            }
          }
        }
        ToolbarItem(placement: .topBarTrailing) {
          Button("Settings", systemImage: "gearshape") { isShowingSettings = true }
        }
      }
      .sheet(isPresented: $isShowingSettings) {
        SettingsView(settings: settings, location: location.coordinate)
      }
      .sheet(item: selectedBike) { bike in
        BikeDetail(
          bike: bike,
          walked: store.nearest.first { $0.id == bike.id },
          controller: controller
        )
        .presentationDetents([.height(320)])
        .presentationBackgroundInteraction(.enabled)
      }
    }
    .onAppear { location.start() }
    .task { await parking.load() }
    .task(id: selectedID) {
      route = nil
      if let bike = selectedBike.wrappedValue, let origin = originCoordinate {
        route = await Handoff.route(from: origin, to: bike)
      }
    }
    .task(id: runKey) {
      guard runKey.isActive, runKey.hasOrigin else { return }
      await controller.run { originCoordinate }
    }
    .onChange(of: scenePhase, initial: true) {
      if scenePhase == .active {
        controller.appDidBecomeActive()
      }
    }
    .onChange(of: origin) {
      if let coordinate = originCoordinate, origin != .here {
        camera = .region(
          MKCoordinateRegion(
            center: coordinate.clLocation, latitudinalMeters: 800, longitudinalMeters: 800))
      } else {
        camera = .userLocation(fallback: .automatic)
      }
    }
  }

  /// The parking bays nearest to the origin.
  private var bays: [ParkingBay] {
    parking.nearest(to: originCoordinate, count: 8)
  }

  private var originCoordinate: Coordinate? {
    origin.coordinate(location: location.coordinate, settings: settings)
  }

  /// The values that restart the refresh loop when they change.
  private struct RunKey: Equatable {
    let isActive: Bool
    let hasOrigin: Bool
    let origin: Origin
    let settings: Settings.Values
  }

  private var runKey: RunKey {
    RunKey(
      // The loop continues in the background while the app watches.
      isActive: scenePhase == .active || controller.watch.isRunning,
      hasOrigin: originCoordinate != nil,
      origin: origin,
      settings: settings.values)
  }

  private var selectedBike: Binding<Bike?> {
    Binding(
      get: { store.bikes.first { $0.id == selectedID } },
      set: { selectedID = $0?.id }
    )
  }
}

/// The bike the rider is walking to.
struct TargetBanner: View {
  let target: Target
  let origin: Coordinate?
  let controller: CommuteController

  var body: some View {
    HStack {
      Image(systemName: target.isReserved ? "clock.badge.checkmark" : "figure.walk")
      if let origin {
        Text(
          "\(Int(distanceM(a: origin, b: target.bike.coordinate))) m to your \(target.isReserved ? "reserved " : "")bike"
        )
      } else {
        Text(target.isReserved ? "Your bike is reserved" : "Walking to your bike")
      }
      Spacer()
      if Handoff.rentalURL(for: target.bike) != nil {
        if !target.isReserved {
          Button("Reserve") {
            controller.reserve(target.bike)
          }
          .buttonStyle(.bordered)
        }
        Button("Unlock") {
          controller.arrived()
          Task { await Handoff.openInMoby(target.bike) }
        }
        .buttonStyle(.borderedProminent)
      }
      Button("Cancel", systemImage: "xmark") {
        controller.arrived()
      }
      .labelStyle(.iconOnly)
      .buttonStyle(.bordered)
    }
    .font(.subheadline)
    .padding(.horizontal)
    .padding(.vertical, 8)
    .background(.green.opacity(0.15))
  }
}

/// Chooses where to search from.
struct OriginPicker: View {
  @Binding var origin: Origin
  let settings: Settings
  let location: Coordinate?

  var body: some View {
    HStack {
      Picker("Search from", selection: $origin) {
        ForEach(Origin.allCases) { origin in
          Text(origin.title).tag(origin)
        }
      }
      .pickerStyle(.segmented)
      Button("Nearest saved place", systemImage: "scope") {
        if let nearest = Origin.nearestPlace(to: location, settings: settings) {
          origin = nearest
        }
      }
      .labelStyle(.iconOnly)
      .disabled(settings.values.places.isEmpty || location == nil)
    }
    .padding(.horizontal)
    .padding(.vertical, 8)
  }
}

struct BikeList: View {
  let store: BikeStore
  let location: LocationProvider
  let origin: Coordinate?
  let nearestBay: ParkingBay?
  @Binding var selectedID: String?

  var body: some View {
    List {
      if let nearestBay {
        Label {
          Text(
            "Nearest parking bay: \(Units.metres(nearestBay.distanceM))"
          )
        } icon: {
          Image(systemName: "parkingsign.circle")
            .foregroundStyle(.blue)
        }
        .font(.subheadline)
      }
      if location.isDenied {
        Text("Location access is off. Turn it on in Settings to find nearby bikes.")
      } else if origin == nil {
        Text("Finding your location…")
      }
      if let error = store.lastError {
        Text(error).foregroundStyle(.red)
      }
      ForEach(store.nearest) { walked in
        Button {
          selectedID = walked.id
        } label: {
          BikeRow(walked: walked)
        }
        .tint(.primary)
      }
      Attribution()
        .listRowSeparator(.hidden)
    }
    .listStyle(.plain)
    .refreshable {
      if let origin {
        await store.refresh(from: origin)
      }
    }
  }
}
