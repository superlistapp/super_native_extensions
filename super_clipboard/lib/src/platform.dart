import 'package:flutter/foundation.dart';

enum ClipboardPlatform {
  web,
  android,
  ios,
  linux,
  macos,
  windows,
}

ClipboardPlatform get currentPlatform {
  if (kIsWeb) {
    return ClipboardPlatform.web;
  } else {
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
        throw UnimplementedError(
            "Unsupported platform: $defaultTargetPlatform");
    }
  }
}
