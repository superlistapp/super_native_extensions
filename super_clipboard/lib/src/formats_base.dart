import 'dart:async';

import 'package:super_clipboard/src/platform.dart';

import 'format.dart';

class SimplePlatformCodec<T extends Object> extends PlatformCodec<T> {
  const SimplePlatformCodec({
    List<PlatformFormat>? formats,
    List<PlatformFormat>? encodingFormats,
    List<PlatformFormat>? decodingFormats,
    this.onDecode,
    this.onEncode,
  })  : _formats = formats,
        _encodingFormats = encodingFormats,
        _decodingFormats = decodingFormats;

  final FutureOr<T?> Function(
      PlatformDataProvider dataProvider, PlatformFormat format)? onDecode;
  final FutureOr<Object?> Function(T value, PlatformFormat format)? onEncode;

  final List<PlatformFormat>? _formats;
  final List<PlatformFormat>? _decodingFormats;
  final List<PlatformFormat>? _encodingFormats;

  @override
  List<PlatformFormat> get encodingFormats =>
      _encodingFormats ?? _formats ?? [];

  @override
  List<PlatformFormat> get decodingFormats =>
      _decodingFormats ?? _formats ?? [];

  @override
  FutureOr<T?> decode(
      PlatformDataProvider dataProvider, PlatformFormat format) async {
    return onDecode != null
        ? onDecode!(dataProvider, format)
        : super.decode(dataProvider, format);
  }

  @override
  FutureOr<Object?> encode(T value, PlatformFormat format) {
    return onEncode != null
        ? onEncode!(value, format)
        : super.encode(value, format);
  }
}

class SimpleDataFormat<T extends Object> extends DataFormat<T> {
  const SimpleDataFormat({
    this.android,
    this.ios,
    this.linux,
    this.macos,
    this.windows,
    this.web,
    this.fallback = const SimplePlatformCodec(formats: []),
  });

  @override
  PlatformCodec<T> codecForPlatform(ClipboardPlatform platform) {
    switch (platform) {
      case ClipboardPlatform.android:
        return android ?? fallback;
      case ClipboardPlatform.ios:
        return ios ?? fallback;
      case ClipboardPlatform.linux:
        return linux ?? fallback;
      case ClipboardPlatform.macos:
        return macos ?? fallback;
      case ClipboardPlatform.windows:
        return windows ?? fallback;
      case ClipboardPlatform.web:
        return web ?? fallback;
    }
  }

  final PlatformCodec<T>? android;
  final PlatformCodec<T>? ios;
  final PlatformCodec<T>? linux;
  final PlatformCodec<T>? macos;
  final PlatformCodec<T>? windows;
  final PlatformCodec<T>? web;
  final PlatformCodec<T> fallback;
}
