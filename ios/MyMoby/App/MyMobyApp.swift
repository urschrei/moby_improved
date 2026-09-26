import SwiftUI

@main
struct MyMobyApp: App {
  @State private var settings: Settings
  @State private var controller: CommuteController
  @State private var location: LocationProvider
  @State private var parking: ParkingStore

  init() {
    let settings = Settings()
    let client = GBFSClient()
    let store = BikeStore(source: Self.source(client: client), settings: settings)
    let location = LocationProvider()
    _settings = State(initialValue: settings)
    _location = State(initialValue: location)
    _parking = State(initialValue: ParkingStore(client: client))
    _controller = State(
      initialValue: CommuteController(store: store, settings: settings, location: location))
    Notifier.shared.register()
  }

  private static func source(client: GBFSClient) -> any FeedSource {
    #if DEBUG
      if CommandLine.arguments.contains("-replay") {
        Log.refresh.info("replaying captured feed responses")
        return ReplaySource()
      }
    #endif
    return client
  }

  var body: some Scene {
    WindowGroup {
      NearbyView(controller: controller, location: location, settings: settings, parking: parking)
        .tint(Theme.accent)
    }
  }
}
