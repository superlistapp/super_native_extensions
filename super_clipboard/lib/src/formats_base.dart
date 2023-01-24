import 'dart:async';

import 'platform.dart';
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

  final Future<T?> Function(
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
  Future<T?> decode(
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

class SimpleFileFormat extends FileFormat {
  final List<PlatformFormat>? androidFormats;
  final List<PlatformFormat>? iosFormats;
  final List<PlatformFormat>? linuxFormats;
  final List<PlatformFormat>? macosFormats;
  final List<PlatformFormat>? windowsFormats;
  final List<PlatformFormat>? webFormats;
  final List<PlatformFormat> fallbackFormats;

  List<PlatformFormat> _formatsForPlatform(ClipboardPlatform platform) {
    switch (platform) {
      case ClipboardPlatform.android:
        return androidFormats ?? fallbackFormats;
      case ClipboardPlatform.ios:
        return iosFormats ?? fallbackFormats;
      case ClipboardPlatform.linux:
        return linuxFormats ?? fallbackFormats;
      case ClipboardPlatform.macos:
        return macosFormats ?? fallbackFormats;
      case ClipboardPlatform.windows:
        return windowsFormats ?? fallbackFormats;
      case ClipboardPlatform.web:
        return webFormats ?? fallbackFormats;
    }
  }

  @override
  PlatformFormat get providerFormat =>
      _formatsForPlatform(currentPlatform).first;

  @override
  List<PlatformFormat> get receiverFormats =>
      _formatsForPlatform(currentPlatform);

  const SimpleFileFormat({
    this.androidFormats,
    this.iosFormats,
    this.linuxFormats,
    this.macosFormats,
    this.windowsFormats,
    this.webFormats,
    this.fallbackFormats = const [],
  });
}

class SimpleValueFormat<T extends Object> extends ValueFormat<T> {
  const SimpleValueFormat({
    this.android,
    this.ios,
    this.linux,
    this.macos,
    this.windows,
    this.web,
    this.fallback = const SimplePlatformCodec(formats: []),
  });

  @override
  PlatformCodec<T> get codec => _codecForPlatform(currentPlatform);

  PlatformCodec<T> _codecForPlatform(ClipboardPlatform platform) {
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
