import SwiftUI

@main
struct MyMobyApp: App {
  @State private var store = BikeStore(source: GBFSClient())
  @State private var location = LocationProvider()

  var body: some Scene {
    WindowGroup {
      NearbyView(store: store, location: location)
    }
  }
}
