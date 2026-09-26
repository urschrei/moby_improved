import MobyKit

/// Bikes that share one marker on the map, because their markers would
/// overlap at the current zoom.
struct MarkerGroup<Item: Identifiable>: Identifiable where Item.ID == String {
  /// The first bike of the group in the input order, which the marker shows.
  let leader: Item
  let members: [Item]

  var id: String { leader.id }
  var count: Int { members.count }

  /// Returns `true` if the bike `id` is in the group.
  func contains(_ id: String?) -> Bool {
    members.contains { $0.id == id }
  }

  /// Groups `items` in their order. Each item joins the first group whose
  /// leader is within `radiusM`, or else starts a group. With the ranked
  /// bikes in order of walking time, each leader is the fastest bike of its
  /// group.
  static func group(
    _ items: [Item], radiusM: Double, coordinate: (Item) -> Coordinate
  ) -> [MarkerGroup<Item>] {
    var leaders: [Item] = []
    var members: [String: [Item]] = [:]
    for item in items {
      if radiusM > 0,
        let leader = leaders.first(where: {
          distanceM(a: coordinate($0), b: coordinate(item)) <= radiusM
        })
      {
        members[leader.id, default: []].append(item)
      } else {
        leaders.append(item)
        members[item.id] = [item]
      }
    }
    return leaders.map { MarkerGroup(leader: $0, members: members[$0.id] ?? [$0]) }
  }
}
