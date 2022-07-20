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
