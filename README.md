# MyMoby

A personal iOS app that shows the nearest available MOBY e-bikes in Dublin, ranked by walking time. It hands over to Apple Maps for walking directions and to the MOBY app to unlock a bike. It uses only the public MOBY GBFS 3.0 feed; it does no renting, unlocking, payment or login.

The logic is in a Rust crate (`core`), exposed to a SwiftUI app through UniFFI (`ffi`).

## Features

- Available bikes with at least a minimum range, ranked by MapKit walking time from the current position or a saved place (home, crèche and two workplaces).
- A map with the nearest bikes, the walking route to a selected bike, and parking bays (the geofencing zones where a ride can end).
- Faster refreshes in commute windows, and a warning when the feed is more than 10 minutes old.
- Background watching with a Live Activity: after **Walk**, the app checks the chosen bike on each refresh. If another rider rents it, the app picks the next nearest bike and sends a notification that opens Maps at it.
- Reports of bikes that the feed lists but that cannot be rented: ghost bikes, and bikes that need service. See [Reporting bikes](#reporting-bikes).
- A **Walk** button on the Live Activity, which chooses the bike it shows and opens Maps. It opens MyMoby first, because a Live Activity link always opens its own app.

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
   just device "YOUR_DEVICE_NAME"
   ```

   To leave out the name, set `MOBY_DEVICE` in your shell profile. Without a name or the variable, the recipe stops before it builds.

6. On the iPhone, trust the developer certificate in **Settings > General > VPN & Device Management**.

### Reinstalling each week

Connect the iPhone and run step 5 again.

### Opening the app at commute time

The app watches in the background only if it starts in the foreground. A Shortcuts automation can open it at the start of a commute window:

1. In Shortcuts, open **Automation** and add a **Time of Day** automation, for example 07:30 on weekdays.
2. Select **Run Immediately**.
3. Add the **Open App** action and choose MyMoby.

In a commute window, the app starts watching when it opens. It stops at the end of the window unless you are walking to a bike.

### Reporting bikes

The app keeps a log of bikes that the feed lists but that you cannot rent, as evidence of the reliability of the feed.

1. Touch and hold a bike on the map or in the list, or tap **Report** on the bike at the top of the sheet.
2. Choose **Ghost Bike** if the bike is not there, or **Needs Service** and a reason if it is there but you cannot rent it. **Needs Servicing, Not Available** is the message that the MOBY app shows.

Each report records the time, the bike number and vehicle ID, the position and range of the bike in the feed, your position and its accuracy, your distance from the bike, and the time of the feed. For 24 hours after a report, the app records whether the feed still lists the bike: when this changes, and every 30 minutes while it does not.

The log is `reports.jsonl`, in the Files app under **On My iPhone › MyMoby**. iCloud Backup includes it. To share the reports, open **Settings** and tap **Export Reports as CSV**. A free Apple ID cannot sign an app that syncs with iCloud, so the log is only on the phone and in its backups.

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
| `just icon` | Draw the app icon from `ios/Icon/MakeIcon.swift` into the asset catalog. |

Run `just ffi` after a change to `core` or `ffi`, and `just project` after adding or removing a Swift file.

### Testing a rented bike

Debug builds accept a `-replay` launch argument. With it, the app serves the feed captured at 11:18 on 26 September 2026, then the one captured at 11:21. Bike `4120848858822619701`, near Pearse Street, is in the first and not in the second.

```bash
xcrun simctl location booted set 53.34375,-6.24690
xcrun simctl launch booted ie.urschrei.mymoby -replay
```

Tap **Walk**. On the next refresh the app replaces it with another bike.

### Testing a failed refresh

Debug builds accept `-fail-first N`. With it, the first `N` refreshes time out, and the app then fetches the live feed. The app tries again after 5 s, 10 s and 20 s.

```bash
xcrun simctl launch booted ie.urschrei.mymoby -fail-first 3
```

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

## The MOBY feed

This section lists the feeds and fields in the MOBY GBFS 3.0 feed, as captured on 26 September 2026 in `core/tests/fixtures/`. The **Used** column shows the fields that the app reads.

The manifest is at `https://moby-move.rideatom.com/gbfs/v3_0/en/gbfs?id=2023`. It lists nine feeds, each with a `name` and a `url`.

### Fields in every feed

| Field | Type | Used | Notes |
|---|---|---|---|
| `last_updated` | string | yes | RFC 3339 time, for example `2026-09-26T11:27:56.918Z`. The app shows the age of `vehicle_status` from it. |
| `ttl` | integer | no | `0` in every feed. |
| `version` | string | no | `3.0`. |
| `data` | object | yes | The content of the feed, as listed below. |

### `vehicle_status`

`data.vehicles` holds one object for each vehicle: 599 in the capture.

| Field | Type | Used | Notes |
|---|---|---|---|
| `vehicle_id` | string | yes | A 19-digit number, for example `3833465119576306229`. |
| `lat`, `lon` | number | yes | The position of the vehicle. |
| `is_reserved` | boolean | yes | `false` for every vehicle in every capture. A reserved or rented vehicle leaves the feed instead. |
| `is_disabled` | boolean | yes | `false` for every vehicle in every capture. |
| `vehicle_type_id` | string | no | `3645` for 598 vehicles and `3646` for one. |
| `current_range_meters` | integer | yes | Missing for the one vehicle of type `3646`. |
| `pricing_plan_id` | string | no | The same value as `vehicle_type_id`. |
| `rental_uris.ios` | string | yes | A Branch link, for example `https://moby-move.app.link/2025070027`. The app opens it with **Open in MOBY**, and shows its last part as the bike number. |
| `rental_uris.android` | string | no | The same link as `rental_uris.ios`. |
| `rental_uris.web` | string | no | `https://moby-move.app.link/` for every vehicle. |

### `vehicle_types`

`data.vehicle_types` holds three types.

| Field | Type | Used | Notes |
|---|---|---|---|
| `vehicle_type_id` | string | no | `3645`, `4178` and `3646`. |
| `form_factor` | string | no | `bicycle` for all three. |
| `propulsion_type` | string | no | `electric_assist` for `3645` and `4178`; `human` for `3646`. |
| `max_range_meters` | integer | no | 50,000 for `3645`. The range gauge uses the same value, set in the app. |
| `default_pricing_plan_id` | string | no | The same value as `vehicle_type_id`. |

### `geofencing_zones`

`data.geofencing_zones` is a GeoJSON feature collection of 1,233 zones, each a `MultiPolygon`. The exterior rings are clockwise.

| Field | Type | Used | Notes |
|---|---|---|---|
| `features[].geometry.coordinates` | array | yes | The polygons of the zone. |
| `features[].properties.rules[].vehicle_type_ids` | array of strings | yes | The types to which the rule applies. |
| `features[].properties.rules[].ride_end_allowed` | boolean | yes | The app shows a zone as a parking bay if its first rule for type `3645` allows the ride to end. 1,184 zones qualify. |
| `features[].properties.rules[].ride_start_allowed` | boolean | no | |
| `features[].properties.rules[].ride_through_allowed` | boolean | no | |
| `global_rules[]` | array | no | One rule: rides cannot start or end outside a zone, but can pass through. |

### `system_pricing_plans`

`data.plans` holds one plan for each vehicle type.

| Field | Type | Used | Notes |
|---|---|---|---|
| `plan_id` | string | no | The same values as `vehicle_type_id`. |
| `name[].text`, `description[].text` | string | no | For example `RM E-bike`, and a text with the maximum distance and price for each ride. |
| `currency` | string | no | `EUR`. |
| `price` | number | no | The price to start a ride: 1.00. |
| `is_taxable` | boolean | no | `false`. |
| `per_min_pricing[].start`, `.rate`, `.interval` | number | no | 0.29 each minute for the e-bikes, 0.04 for type `3646`. |

### `system_information`

| Field | Type | Used | Notes |
|---|---|---|---|
| `system_id` | string | no | `2023`. |
| `name[].text` | string | no | `Dublin`. |
| `languages` | array of strings | no | `en`. |
| `timezone` | string | no | `Europe/Dublin`. The commute schedule uses the time zone of the phone. |
| `opening_hours` | string | no | `24/7`. |
| `feed_contact_email` | string | no | Empty. The GBFS schema requires an email address, so this feed does not validate. |
| `rental_apps.ios.store_uri`, `.discovery_uri` | string | no | The App Store page, and `https://moby-move.app.link/`. |
| `rental_apps.android.store_uri`, `.discovery_uri` | string | no | The Google Play page, and the same Branch link. |

### Other feeds

| Feed | Contents |
|---|---|
| `gbfs` | The manifest: `data.feeds[]`, each with `name` and `url`. The app finds `vehicle_status` and `geofencing_zones` in it. |
| `gbfs_versions` | `data.versions[]`: versions 2.2 and 3.0, each with `version` and `url`. |
| `station_information`, `station_status` | `data.stations` is empty. MOBY is dockless. |

## Data

Bike data: [Moby Bikes API](https://data.smartdublin.ie/dataset/moby-bikes), Dublin City Council / Smart Dublin, [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).

## Licence

[Blue Oak Model License 1.0.0](LICENSE.md). The bike data has its own licence: see [Data](#data).
