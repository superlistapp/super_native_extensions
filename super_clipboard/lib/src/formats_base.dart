import 'dart:async';

import 'format.dart';
import 'platform.dart';

class SimplePlatformCodec<T> implements PlatformCodec<T> {
  const SimplePlatformCodec({
    required this.onDecode,
    required this.onEncode,
    required this.formats,
  });

  final FutureOr<T> Function(Object value, PlatformFormat format) onDecode;
  final FutureOr<Object> Function(T value, PlatformFormat format) onEncode;

  final List<PlatformFormat> formats;

  @override
  List<PlatformFormat> get encodableFormats => formats;

  @override
  List<PlatformFormat> get decodableFormats => formats;

  @override
  FutureOr<T> decode(Object data, PlatformFormat format) {
    return onDecode(data, format);
  }

  @override
  FutureOr<Object> encode(T t, PlatformFormat format) {
    return onEncode(t, format);
  }
}

class FallbackPlatformCodec<T> implements PlatformCodec<T> {
  const FallbackPlatformCodec();

  @override
  FutureOr<T> decode(Object data, PlatformFormat format) {
    throw UnimplementedError();
  }

  @override
  FutureOr<Object> encode(T t, PlatformFormat format) {
    throw UnimplementedError();
  }

  @override
  List<PlatformFormat> get encodableFormats => [];

  @override
  List<PlatformFormat> get decodableFormats => [];
}

class SimpleDataFormat<T> extends EncodableDataFormat<T> {
  const SimpleDataFormat({
    this.android,
    this.ios,
    this.linux,
    this.macos,
    this.windows,
    this.web,
  });

  SimpleDataFormat.passthrough({
    PlatformFormat? android,
    PlatformFormat? ios,
    PlatformFormat? linux,
    PlatformFormat? macos,
    PlatformFormat? windows,
    PlatformFormat? web,
    PlatformFormat? defaultFormat,
  })  : android = _passthroughCodec(android ?? defaultFormat),
        ios = _passthroughCodec(ios ?? defaultFormat),
        linux = _passthroughCodec(linux ?? defaultFormat),
        macos = _passthroughCodec(macos ?? defaultFormat),
        windows = _passthroughCodec(windows ?? defaultFormat),
        web = _passthroughCodec(web ?? defaultFormat);

  static PlatformCodec<T>? _passthroughCodec<T>(PlatformFormat? format) {
    return format != null
        ? SimplePlatformCodec<T>(
            onDecode: (t, _) => t as T,
            onEncode: (v, _) => v as Object,
            formats: [format])
        : null;
  }

  final PlatformCodec<T>? android;
  final PlatformCodec<T>? ios;
  final PlatformCodec<T>? linux;
  final PlatformCodec<T>? macos;
  final PlatformCodec<T>? windows;
  final PlatformCodec<T>? web;

  @override
  PlatformCodec<T> codecForPlatform(ClipboardPlatform platform) {
    switch (platform) {
      case ClipboardPlatform.android:
        return android ?? const FallbackPlatformCodec();
      case ClipboardPlatform.ios:
        return ios ?? const FallbackPlatformCodec();
      case ClipboardPlatform.linux:
        return linux ?? const FallbackPlatformCodec();
      case ClipboardPlatform.macos:
        return macos ?? const FallbackPlatformCodec();
      case ClipboardPlatform.windows:
        return windows ?? const FallbackPlatformCodec();
      case ClipboardPlatform.web:
        return web ?? const FallbackPlatformCodec();
    }
  }
}
