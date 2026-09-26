import SwiftUI

@main
struct MyMobyApp: App {
  @State private var settings: Settings
  @State private var store: BikeStore
  @State private var location = LocationProvider()

  init() {
    let settings = Settings()
    _settings = State(initialValue: settings)
    _store = State(initialValue: BikeStore(source: GBFSClient(), settings: settings))
  }

  var body: some Scene {
    WindowGroup {
      NearbyView(store: store, location: location, settings: settings)
    }
  }
}
