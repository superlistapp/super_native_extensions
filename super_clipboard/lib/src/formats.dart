import 'dart:async';

import 'format_conversions.dart';
import 'formats_base.dart';
import 'platform.dart';

// These types will be converted to CF constants with number
// appended to the prefix
const cfInternalPrefix = 'NativeShell_InternalWindowsFormat_';

const formatPlainText = SimpleDataFormat<String>(
  ios: SimplePlatformFormat<String>(
    formats: ['public.utf8-plain-text'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  macos: SimplePlatformFormat<String>(
    formats: ['public.utf8-plain-text'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  android: SimplePlatformFormat<String>(
    formats: ['text/plain'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  linux: SimplePlatformFormat<String>(
    formats: ['text/plain'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  windows: SimplePlatformFormat(
    formats: [
      '${cfInternalPrefix}13' // CF_UNICODETEXT
    ],
    onDecode: fromSystemUtf16NullTerminated,
    onEncode: passthrough,
  ),
  web: SimplePlatformFormat(
    formats: ['text/plain'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
);

/// Key for rich text in form of html snippet. Make sure to include `formatPlainText`
/// version in clipboard as well, otherwise setting the content may fail on some
/// platforms (i.e. Android).
const formatHtml = SimpleDataFormat<String>(
  ios: SimplePlatformFormat<String>(
    formats: ['public.html'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  macos: SimplePlatformFormat<String>(
    formats: ['public.html'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  android: SimplePlatformFormat<String>(
    formats: ['text/html'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  linux: SimplePlatformFormat<String>(
    formats: ['text/html'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  windows: SimplePlatformFormat<String>(
    onDecode: windowsHtmlFromSystem,
    onEncode: windowsHtmlToSystem,
    formats: [
      'text/html',
      cfHtml,
    ],
  ),
);

class CustomDataFormat<T> extends BaseDataFormat {
  CustomDataFormat(
    this.applicationId, {
    FutureOr<T> Function(Object value, String platformType)? onDecode,
    this.onEncode = passthrough,
  }) : onDecode = onDecode ?? _fallbackConvertFromSystem<T>;

  final String applicationId;

  final FutureOr<T> Function(Object value, String platformType) onDecode;
  final FutureOr<Object> Function(T value, String platformType) onEncode;

  static T _fallbackConvertFromSystem<T>(Object value, String platformType) {
    if (value is T) {
      return value as T;
    } else {
      throw FormatException('Unsupported value type: ${value.runtimeType}');
    }
  }

  @override
  PlatformFormat formatForPlatform(ClipboardPlatform platform) {
    switch (platform) {
      case ClipboardPlatform.android:
        return SimplePlatformFormat<T>(
            onDecode: onDecode,
            onEncode: onEncode,
            formats: ["application/x-private;appId=$applicationId"]);
      case ClipboardPlatform.ios:
        return SimplePlatformFormat<T>(
            onDecode: onDecode, onEncode: onEncode, formats: [applicationId]);
      case ClipboardPlatform.linux:
        return SimplePlatformFormat<T>(
            onDecode: onDecode,
            onEncode: onEncode,
            formats: ["application/x-private;appId=$applicationId"]);
      case ClipboardPlatform.macos:
        return SimplePlatformFormat<T>(
            onDecode: onDecode, onEncode: onEncode, formats: [applicationId]);
      case ClipboardPlatform.windows:
        return SimplePlatformFormat<T>(
            onDecode: onDecode, onEncode: onEncode, formats: [applicationId]);
      case ClipboardPlatform.web:
        throw UnsupportedError('Custom values are not supported on web.');
    }
  }
}
