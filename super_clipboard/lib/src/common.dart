import 'dart:async';

import 'package:flutter/foundation.dart';

enum ClipboardPlatform {
  android,
  ios,
  linux,
  macos,
  windows,
}

ClipboardPlatform get _currentPlatform {
  switch (defaultTargetPlatform) {
    case TargetPlatform.android:
      return ClipboardPlatform.android;
    case TargetPlatform.iOS:
      return ClipboardPlatform.ios;
    case TargetPlatform.linux:
      return ClipboardPlatform.linux;
    case TargetPlatform.macOS:
      return ClipboardPlatform.macos;
    case TargetPlatform.windows:
      return ClipboardPlatform.windows;
    default:
      throw UnimplementedError("Unsupported platform: $defaultTargetPlatform");
  }
}

abstract class ClipboardKey<T> {
  const ClipboardKey();

  ClipboardPlatformKey<T> keyForPlatform(ClipboardPlatform platform);
  ClipboardPlatformKey<T> platformKey() => keyForPlatform(_currentPlatform);
}

abstract class ClipboardPlatformKey<T> {
  const ClipboardPlatformKey();

  FutureOr<T?> convertFromSystem(Object value, String platformType);
  FutureOr<Object> convertToSystem(T value, String platformType);
  List<String> readableSystemTypes();
  List<String> writableSystemTypes();
}
