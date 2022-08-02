import 'dart:async';

import 'format.dart';
import 'platform.dart';

abstract class PlatformFormat<T>
    implements PlatformEncoder<T>, PlatformDecoder<T> {}

class SimplePlatformFormat<T> implements PlatformFormat<T> {
  const SimplePlatformFormat({
    required this.onDecode,
    required this.onEncode,
    required this.formats,
  });

  final FutureOr<T> Function(Object value, String platformType) onDecode;
  final FutureOr<Object> Function(T value, String platformType) onEncode;

  @override
  final List<String> formats;

  @override
  bool canDecode(String platformFormat) {
    return formats.contains(platformFormat);
  }

  @override
  FutureOr<T> decode(Object data, String format) {
    return onDecode(data, format);
  }

  @override
  FutureOr<Object> encode(T t, String format) {
    return onEncode(t, format);
  }
}

class FallbackPlatformFormat<T> implements PlatformFormat<T> {
  const FallbackPlatformFormat();

  @override
  bool canDecode(String format) {
    return false;
  }

  @override
  FutureOr<T> decode(Object data, String format) {
    throw UnimplementedError();
  }

  @override
  FutureOr<Object> encode(T t, String format) {
    throw UnimplementedError();
  }

  @override
  List<String> get formats => [];
}

abstract class BaseDataFormat<T> extends DataFormat<T> {
  const BaseDataFormat();

  PlatformFormat<T> formatForPlatform(ClipboardPlatform platform);

  @override
  PlatformDecoder<T> decoderForPlatform(ClipboardPlatform platform) {
    return formatForPlatform(platform);
  }

  @override
  PlatformEncoder<T> encoderForPlatform(ClipboardPlatform platform) {
    return formatForPlatform(platform);
  }
}

class SimpleDataFormat<T> extends BaseDataFormat<T> {
  const SimpleDataFormat({
    this.android,
    this.ios,
    this.linux,
    this.macos,
    this.windows,
    this.web,
  });

  final PlatformFormat<T>? android;
  final PlatformFormat<T>? ios;
  final PlatformFormat<T>? linux;
  final PlatformFormat<T>? macos;
  final PlatformFormat<T>? windows;
  final PlatformFormat<T>? web;

  @override
  PlatformFormat<T> formatForPlatform(ClipboardPlatform platform) {
    switch (platform) {
      case ClipboardPlatform.android:
        return android ?? const FallbackPlatformFormat();
      case ClipboardPlatform.ios:
        return ios ?? const FallbackPlatformFormat();
      case ClipboardPlatform.linux:
        return linux ?? const FallbackPlatformFormat();
      case ClipboardPlatform.macos:
        return macos ?? const FallbackPlatformFormat();
      case ClipboardPlatform.windows:
        return windows ?? const FallbackPlatformFormat();
      case ClipboardPlatform.web:
        return web ?? const FallbackPlatformFormat();
    }
  }
}
