import 'package:flutter/rendering.dart';

/// Represents currently running long-press interaction. The interaction starts at long-press
/// and extends all the way until menu dismissal or end of drag and drop.
class LongPressSession {
  static int _depth = 0;

  static bool get active => _depth > 0;

  /// Register cleanup callback executed when current long press session ends.
  static void onCleanup(VoidCallback cb) {
    assert(_depth > 0);
    _cleanupCallbacks.add(cb);
  }

  static final _cleanupCallbacks = <VoidCallback>[];

  /// Runs the action block from within a long press session.
  static T run<T>(T Function() action) {
    _depth++;
    try {
      return action();
    } finally {
      _depth--;
      if (_depth == 0) {
        _cleanup();
      }
    }
  }

  /// Extends the length of current long press session until the future completes.
  static Future<T> extend<T>(Future<T> Function() future) async {
    _depth++;
    try {
      return await future();
    } finally {
      _depth--;
      if (_depth == 0) {
        _cleanup();
      }
    }
  }

  static void _cleanup() {
    for (final cb in _cleanupCallbacks) {
      cb();
    }
    _cleanupCallbacks.clear();
  }
}
