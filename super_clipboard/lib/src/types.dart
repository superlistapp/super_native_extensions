import 'dart:async';

import 'common.dart';
import 'types_internal.dart';

const typePlaintext = SimpleClipboardType<String>(
  ios: SimpleClipboardPlatformType<String>(
    types: ['public.utf8-plain-text'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  macos: SimpleClipboardPlatformType<String>(
    types: ['public.utf8-plain-text'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  android: SimpleClipboardPlatformType<String>(
    types: ['text/plain'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  linux: SimpleClipboardPlatformType<String>(
    types: ['text/plain'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  windows: SimpleClipboardPlatformType(
      onConvertFromSystem: fromSystemUtf16NullTerminated,
      onConvertToSystem: passthrough,
      types: [
        '${cfInternalPrefix}13' // CF_UNICODETEXT
      ]),
);

/// Key for rich text in form of html snippet. Make sure to include `plaintextKey`
/// version in clipboard as well, otherwise setting the content may fail on some
/// platforms (i.e. Android).
const typeHtml = SimpleClipboardType<String>(
  ios: SimpleClipboardPlatformType<String>(
    types: ['public.html'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  macos: SimpleClipboardPlatformType<String>(
    types: ['public.html'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  android: SimpleClipboardPlatformType<String>(
    types: ['text/html'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  linux: SimpleClipboardPlatformType<String>(
    types: ['text/html'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  windows: SimpleClipboardPlatformType(
      onConvertFromSystem: windowsHtmlFromSystem,
      onConvertToSystem: windowsHtmlToSystem,
      types: [
        'text/html',
        cfHtml,
      ]),
);

class CustomClipboardType<T> extends ClipboardType<T> {
  final String applicationId;

  late final FutureOr<T?> Function(Object value, String platformType)
      onConvertFromSystem;
  final FutureOr<Object> Function(T value, String platformType)
      onConvertToSystem;

  T? _fallbackConvertFromSystem(Object value, String platformType) {
    return value is T ? value as T : null;
  }

  CustomClipboardType(
    this.applicationId, {
    FutureOr<T?> Function(Object value, String platformType)?
        onConvertFromSystem,
    this.onConvertToSystem = passthrough,
  }) {
    this.onConvertFromSystem =
        onConvertFromSystem ?? _fallbackConvertFromSystem;
  }

  @override
  ClipboardPlatformType<T> platformTypeFor(ClipboardPlatform platform) {
    switch (platform) {
      case ClipboardPlatform.android:
        return SimpleClipboardPlatformType(
            onConvertFromSystem: onConvertFromSystem,
            onConvertToSystem: onConvertToSystem,
            types: ["application/x-private;appId=$applicationId"]);
      case ClipboardPlatform.ios:
        return SimpleClipboardPlatformType(
            onConvertFromSystem: onConvertFromSystem,
            onConvertToSystem: onConvertToSystem,
            types: [applicationId]);
      case ClipboardPlatform.linux:
        return SimpleClipboardPlatformType(
            onConvertFromSystem: onConvertFromSystem,
            onConvertToSystem: onConvertToSystem,
            types: ["application/x-private;appId=$applicationId"]);
      case ClipboardPlatform.macos:
        return SimpleClipboardPlatformType(
            onConvertFromSystem: onConvertFromSystem,
            onConvertToSystem: onConvertToSystem,
            types: [applicationId]);
      case ClipboardPlatform.windows:
        return SimpleClipboardPlatformType(
            onConvertFromSystem: onConvertFromSystem,
            onConvertToSystem: onConvertToSystem,
            types: [applicationId]);
    }
  }
}
