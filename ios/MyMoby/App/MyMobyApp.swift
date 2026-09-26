import SwiftUI

@main
struct MyMobyApp: App {
  @State private var store = BikeStore(source: FixtureSource())

  var body: some Scene {
    WindowGroup {
      BikeListView(store: store)
    }
  }
}
