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

abstract class ClipboardType<T> {
  const ClipboardType();

  ClipboardPlatformType<T> platformTypeFor(ClipboardPlatform platform);
  ClipboardPlatformType<T> platformType() => platformTypeFor(_currentPlatform);
}

abstract class ClipboardPlatformType<T> {
  const ClipboardPlatformType();

  FutureOr<T?> convertFromSystem(Object value, String platformType);
  FutureOr<Object> convertToSystem(T value, String platformType);
  List<String> readableSystemTypes();
  List<String> writableSystemTypes();
}
