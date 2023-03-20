import 'dart:async';
import 'dart:ui';

import 'package:flutter/foundation.dart';

class SimpleNotifier extends ChangeNotifier {
  void notify() {
    super.notifyListeners();
  }
}

extension RectExt on Rect {
  Map serialize() => {
        'x': left,
        'y': top,
        'width': width,
        'height': height,
      };
  static Rect deserialize(dynamic rect) {
    final map = rect as Map;
    return Rect.fromLTWH(map['x'], map['y'], map['width'], map['height']);
  }
}

extension OffsetExt on Offset {
  Map serialize() => {
        'x': dx,
        'y': dy,
      };
  static Offset deserialize(dynamic position) {
    final map = position as Map;
    return Offset(map['x'], map['y']);
  }
}

extension SizeExt on Size {
  Map serialize() => {
        'width': width,
        'height': height,
      };
  static Size deserialize(dynamic size) {
    final map = size as Map;
    return Size(map['width'], map['height']);
  }
}

extension DurationExt on Duration {
  double get inSecondsDouble => inMicroseconds / 1000000.0;

  static Duration fromSeconds(double seconds) =>
      Duration(microseconds: (seconds * 1000000.0).round());
}

// For cases where platform code doesn't really care about dart exceptions.
// Instead pass it to default handler and return sane default.
Future<T> handleError<T>(Future<T> Function() f, T Function() def) async {
  try {
    return await f();
  } catch (e, s) {
    Zone.current.handleUncaughtError(e, s);
    return def();
  }
}
