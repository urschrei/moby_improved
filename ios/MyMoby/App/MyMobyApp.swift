import SwiftUI

@main
struct MyMobyApp: App {
  @State private var settings: Settings
  @State private var controller: CommuteController
  @State private var location: LocationProvider

  init() {
    let settings = Settings()
    let store = BikeStore(source: Self.source(), settings: settings)
    let location = LocationProvider()
    _settings = State(initialValue: settings)
    _location = State(initialValue: location)
    _controller = State(
      initialValue: CommuteController(store: store, settings: settings, location: location))
    Notifier.shared.register()
  }

  private static func source() -> any FeedSource {
    #if DEBUG
      if CommandLine.arguments.contains("-replay") {
        Log.refresh.info("replaying captured feed responses")
        return ReplaySource()
      }
    #endif
    return GBFSClient()
  }

  var body: some Scene {
    WindowGroup {
      NearbyView(controller: controller, location: location, settings: settings)
    }
  }
}
