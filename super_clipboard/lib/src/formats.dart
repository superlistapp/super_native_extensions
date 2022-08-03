import 'dart:async';

import 'package:super_clipboard/src/format.dart';

import 'format_conversions.dart';
import 'formats_base.dart';
import 'platform.dart';

// These types will be converted to CF constants with number
// appended to the prefix
const cfInternalPrefix = 'NativeShell_InternalWindowsFormat_';

const formatPlainText = SimpleDataFormat<String>(
  ios: SimplePlatformCodec<String>(
    formats: ['public.utf8-plain-text'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  macos: SimplePlatformCodec<String>(
    formats: ['public.utf8-plain-text'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  android: SimplePlatformCodec<String>(
    formats: ['text/plain'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  linux: SimplePlatformCodec<String>(
    formats: ['text/plain'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  windows: SimplePlatformCodec(
    formats: [
      '${cfInternalPrefix}13' // CF_UNICODETEXT
    ],
    onDecode: fromSystemUtf16NullTerminated,
    onEncode: passthrough,
  ),
  web: SimplePlatformCodec(
    formats: ['text/plain'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
);

/// Key for rich text in form of html snippet. Make sure to include `formatPlainText`
/// version in clipboard as well, otherwise setting the content may fail on some
/// platforms (i.e. Android).
const formatHtml = SimpleDataFormat<String>(
  ios: SimplePlatformCodec<String>(
    formats: ['public.html'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  macos: SimplePlatformCodec<String>(
    formats: ['public.html'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  android: SimplePlatformCodec<String>(
    formats: ['text/html'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  linux: SimplePlatformCodec<String>(
    formats: ['text/html'],
    onDecode: fromSystemUtf8,
    onEncode: passthrough,
  ),
  windows: SimplePlatformCodec<String>(
    onDecode: windowsHtmlFromSystem,
    onEncode: windowsHtmlToSystem,
    formats: [
      'text/html',
      cfHtml,
    ],
  ),
);

class CustomDataFormat<T> extends EncodableDataFormat {
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
  PlatformCodec codecForPlatform(ClipboardPlatform platform) {
    switch (platform) {
      case ClipboardPlatform.android:
        return SimplePlatformCodec<T>(
            onDecode: onDecode,
            onEncode: onEncode,
            formats: ["application/x-private;appId=$applicationId"]);
      case ClipboardPlatform.ios:
        return SimplePlatformCodec<T>(
            onDecode: onDecode, onEncode: onEncode, formats: [applicationId]);
      case ClipboardPlatform.linux:
        return SimplePlatformCodec<T>(
            onDecode: onDecode,
            onEncode: onEncode,
            formats: ["application/x-private;appId=$applicationId"]);
      case ClipboardPlatform.macos:
        return SimplePlatformCodec<T>(
            onDecode: onDecode, onEncode: onEncode, formats: [applicationId]);
      case ClipboardPlatform.windows:
        return SimplePlatformCodec<T>(
            onDecode: onDecode, onEncode: onEncode, formats: [applicationId]);
      case ClipboardPlatform.web:
        throw UnsupportedError('Custom values are not supported on web.');
    }
  }
}
