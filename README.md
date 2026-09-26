# MyMoby

A personal iOS app that shows the nearest available MOBY e-bikes in Dublin, ranked by walking time. It hands over to Apple Maps for walking directions and to the MOBY app to unlock a bike. It uses only the public MOBY GBFS 3.0 feed; it does no renting, unlocking, payment or login.

The logic is in a Rust crate (`core`), exposed to a SwiftUI app through UniFFI (`ffi`).

## Features

- Available bikes with at least a minimum range, ranked by MapKit walking time from the current position or a saved place (home, crèche and two workplaces).
- A map with the nearest bikes, the walking route to a selected bike, and parking bays (the geofencing zones where a ride can end).
- Faster refreshes in commute windows, and a warning when the feed is more than 10 minutes old.
- Background watching with a Live Activity: after **Walk there**, the app checks the chosen bike on each refresh. If another rider rents it, the app picks the next nearest bike and sends a notification that opens Maps at it.

## Requirements

- macOS with Xcode 26 and an iOS 18 simulator runtime
- Rust 1.88 or later, with the `aarch64-apple-ios` and `aarch64-apple-ios-sim` targets
- [`just`](https://github.com/casey/just), [`cargo-nextest`](https://nexte.st) and [XcodeGen](https://github.com/yonaskolb/XcodeGen)
- For the phone: an iPhone on iOS 18 and an Apple ID. A paid developer account is not necessary.

## Installing on an iPhone

A free Apple ID signs the app for 7 days. After that, the app does not open until you install it again.

### Setting up once

1. In Xcode, add your Apple ID in **Settings > Accounts**. Note the team ID of your personal team.
2. Create `ios/Local.xcconfig` (it is not in version control):

   ```
   DEVELOPMENT_TEAM = YOUR_TEAM_ID
   MOBY_BUNDLE_ID = ie.YOUR_NAME.mymoby
   ```

3. Connect the iPhone and turn on **Settings > Privacy & Security > Developer Mode**.
4. Find the device name:

   ```bash
   xcrun devicectl list devices
   ```

5. Build and install:

   ```bash
   MOBY_DEVICE="YOUR_DEVICE_NAME" just device
   ```

6. On the iPhone, trust the developer certificate in **Settings > General > VPN & Device Management**.

### Reinstalling each week

Connect the iPhone and run step 5 again.

### Opening the app at commute time

The app watches in the background only if it starts in the foreground. A Shortcuts automation can open it at the start of a commute window:

1. In Shortcuts, open **Automation** and add a **Time of Day** automation, for example 07:30 on weekdays.
2. Select **Run Immediately**.
3. Add the **Open App** action and choose MyMoby.

In a commute window, the app starts watching when it opens. It stops at the end of the window unless you are walking to a bike.

### If Open in MOBY goes to the App Store

The rental links are Branch universal links on `moby-move.app.link`. If iOS has recorded a choice to open that domain in Safari, the link goes to the App Store instead of the MOBY app. To open the domain in the MOBY app again:

1. Paste a rental link, for example `https://moby-move.app.link/2025070027`, into a note in Notes.
2. Touch and hold the link, then tap **Open in MOBY**.

## Developing

| Command | Action |
|---|---|
| `just test` | Run the Rust tests. |
| `just ffi` | Build the Rust library for iOS, the simulator and macOS, and regenerate the Swift bindings. |
| `just swift-test` | Run the Swift smoke tests against the macOS build of the library. |
| `just app` | Build the app for the iOS 18 simulator. |
| `just project` | Regenerate `ios/MyMoby.xcodeproj` from `ios/project.yml`. |
| `just swift-fmt` | Format the Swift sources. |

Run `just ffi` after a change to `core` or `ffi`, and `just project` after adding or removing a Swift file.

### Testing a rented bike

Debug builds accept a `-replay` launch argument. With it, the app serves the feed captured at 11:18 on 26 September 2026, then the one captured at 11:21. Bike `4120848858822619701`, near Pearse Street, is in the first and not in the second.

```bash
xcrun simctl location booted set 53.34375,-6.24690
xcrun simctl launch booted ie.urschrei.mymoby -replay
```

Choose the nearest bike and tap **Walk there**. On the next refresh the app replaces it with another bike.

## Layout

| Path | Contents |
|---|---|
| `core/` | GBFS types, candidate filtering, walking-distance search, commute schedule, target tracking, parking index |
| `core/tests/fixtures/` | Captured feed responses |
| `core/tests/schemas/` | Official GBFS 3.0 JSON schemas |
| `ffi/` | UniFFI bindings |
| `ios/MobyKit/` | Swift package with the XCFramework and the generated bindings |
| `ios/MyMoby/` | SwiftUI app |
| `ios/MyMobyWidgets/` | Live Activity |

## Data

Bike data: [Moby Bikes API](https://data.smartdublin.ie/dataset/moby-bikes), Dublin City Council / Smart Dublin, [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
