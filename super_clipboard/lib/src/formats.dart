// These types will be converted to CF constants with number
// appended to the prefix

import 'dart:async';
import 'dart:typed_data';

import 'package:super_clipboard/src/platform.dart';

import 'format.dart';
import 'formats_base.dart';
import 'format_conversions.dart';

const cfInternalPrefix = 'NativeShell_InternalWindowsFormat_';

class Format {
  static const plainText = SimpleDataFormat<String>(
    ios: SimplePlatformCodec(
      formats: ['public.utf8-plain-text'],
      onDecode: fromSystemUtf8,
    ),
    macos: SimplePlatformCodec(
      formats: ['public.utf8-plain-text'],
      onDecode: fromSystemUtf8,
    ),
    windows: SimplePlatformCodec(
      formats: [
        '${cfInternalPrefix}13' // CF_UNICODETEXT
      ],
      onDecode: fromSystemUtf16NullTerminated,
    ),
    // other platforms
    fallback: SimplePlatformCodec(
      formats: ['text/plain'],
      onDecode: fromSystemUtf8,
    ),
  );

  /// Key for rich text in form of html snippet. Make sure to include `formatPlainText`
  /// version in clipboard as well, otherwise setting the content may fail on some
  /// platforms (i.e. Android).
  static const html = SimpleDataFormat<String>(
    ios: SimplePlatformCodec<String>(
      formats: ['public.html'],
      onDecode: fromSystemUtf8,
    ),
    macos: SimplePlatformCodec<String>(
      formats: ['public.html'],
      onDecode: fromSystemUtf8,
    ),
    windows: SimplePlatformCodec<String>(
      onDecode: windowsHtmlFromSystem,
      onEncode: windowsHtmlToSystem,
      formats: [
        'text/html',
        cfHtml,
      ],
    ),
    fallback: SimplePlatformCodec<String>(
      formats: ['text/html'],
      onDecode: fromSystemUtf8,
    ),
  );

  static const fileUri = SimpleDataFormat<Uri>(
    ios: SimplePlatformCodec<Uri>(
      formats: ['public.file-url'],
      onDecode: fileUriFromString,
      onEncode: fileUriToString,
    ),
    macos: SimplePlatformCodec<Uri>(
      formats: ['public.file-url'],
      onDecode: fileUriFromString,
      onEncode: fileUriToString,
    ),
    windows: SimplePlatformCodec<Uri>(
      formats: [
        '${cfInternalPrefix}15' // CF_HDROP
      ],
      onDecode: fileUriFromWindowsPath,
      onEncode: fileUriToWindowsPath,
    ),
    fallback: SimplePlatformCodec<Uri>(
      formats: ['text/uri-list'],
      onDecode: fileUriFromString,
      onEncode: fileUriToString,
    ),
  );

  static const imageJpeg = SimpleDataFormat<Uint8List>(
    macos: SimplePlatformCodec(formats: ['public.jpeg']),
    ios: SimplePlatformCodec(formats: ['public.jpeg']),
    fallback: SimplePlatformCodec(formats: ['image/jpeg']),
  );

  static const imagePng = SimpleDataFormat<Uint8List>(
    macos: SimplePlatformCodec(formats: ['public.png']),
    ios: SimplePlatformCodec(formats: ['public.png']),
    windows: SimplePlatformCodec(formats: ['PNG']),
    fallback: SimplePlatformCodec(formats: ['image/png']),
  );

  static const imageGif = SimpleDataFormat<Uint8List>(
    macos: SimplePlatformCodec(formats: ['public.gif']),
    ios: SimplePlatformCodec(formats: ['public.gif']),
    fallback: SimplePlatformCodec(formats: ['image/gif']),
  );

  static const imageTiff = SimpleDataFormat<Uint8List>(
    macos: SimplePlatformCodec(formats: ['public.tiff']),
    ios: SimplePlatformCodec(formats: ['public.tiff']),
    fallback: SimplePlatformCodec(formats: ['image/tiff']),
  );

  static const imageWebP = SimpleDataFormat<Uint8List>(
    macos: SimplePlatformCodec(formats: ['org.webmproject.webp']),
    ios: SimplePlatformCodec(formats: ['org.webmproject.webp']),
    fallback: SimplePlatformCodec(formats: ['image/webp']),
  );

  static const imageSvg = SimpleDataFormat<Uint8List>(
    macos: SimplePlatformCodec(formats: ['public.svg-image']),
    ios: SimplePlatformCodec(formats: ['public.svg-image']),
    fallback: SimplePlatformCodec(formats: ['image/svg+xml']),
  );

  static const imageBmp = SimpleDataFormat<Uint8List>(
    macos: SimplePlatformCodec(formats: ['com.microsoft.bmp']),
    ios: SimplePlatformCodec(formats: ['com.microsoft.bmp']),
    fallback: SimplePlatformCodec(formats: ['image/bmp']),
  );
}

class CustomDataFormat<T extends Object> extends DataFormat<T> {
  final String applicationId;
  final FutureOr<T?> Function(Object value, String platformType)? onDecode;
  final FutureOr<Object> Function(T value, String platformType)? onEncode;

  CustomDataFormat({
    required this.applicationId,
    this.onDecode,
    this.onEncode,
  });

  @override
  PlatformCodec<T> codecForPlatform(ClipboardPlatform platform) {
    switch (platform) {
      case ClipboardPlatform.android:
        return SimplePlatformCodec<T>(
          formats: ["application/x-private;appId=$applicationId"],
          onDecode: onDecode,
          onEncode: onEncode,
        );
      case ClipboardPlatform.ios:
        return SimplePlatformCodec<T>(
          formats: [applicationId],
          onDecode: onDecode,
          onEncode: onEncode,
        );
      case ClipboardPlatform.macos:
        return SimplePlatformCodec<T>(
          formats: [applicationId],
          onDecode: onDecode,
          onEncode: onEncode,
        );
      case ClipboardPlatform.linux:
        return SimplePlatformCodec<T>(
          formats: ["application/x-private;appId=$applicationId"],
          onDecode: onDecode,
          onEncode: onEncode,
        );
      case ClipboardPlatform.windows:
        return SimplePlatformCodec<T>(
          formats: [applicationId],
          onDecode: onDecode,
          onEncode: onEncode,
        );
      case ClipboardPlatform.web:
        return const SimplePlatformCodec(formats: []);
    }
  }
}
