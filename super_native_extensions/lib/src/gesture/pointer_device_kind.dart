import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';

class PointerDeviceKindDetector {
  static final instance = PointerDeviceKindDetector._();

  ValueListenable<PointerDeviceKind> get current => _current;

  final _current = ValueNotifier(_defaultDeviceKind());

  PointerDeviceKindDetector._() {
    GestureBinding.instance.pointerRouter
        .addGlobalRoute(_handleGlobalPointerEvent);
  }

  void _handleGlobalPointerEvent(PointerEvent event) {
    _current.value = event.kind;
  }

  static PointerDeviceKind _defaultDeviceKind() {
    return defaultTargetPlatform == TargetPlatform.iOS ||
            defaultTargetPlatform == TargetPlatform.android
        ? PointerDeviceKind.touch
        : PointerDeviceKind.mouse;
  }
}
