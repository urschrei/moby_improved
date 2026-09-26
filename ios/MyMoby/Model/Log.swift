import os

enum Log {
  static let refresh = Logger(subsystem: "ie.urschrei.mymoby", category: "refresh")
  static let routing = Logger(subsystem: "ie.urschrei.mymoby", category: "routing")
}
