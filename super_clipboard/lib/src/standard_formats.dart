import 'dart:async';
import 'dart:typed_data';

import 'format.dart';
import 'formats_base.dart';
import 'format_conversions.dart';
import 'platform.dart';

// These types will be converted to CF constants with number
// appended to the prefix
const cfInternalPrefix = 'NativeShell_CF_';
const cfUnicodeText = '${cfInternalPrefix}13';
const cfHdrop = '${cfInternalPrefix}15';
const cfTiff = '${cfInternalPrefix}6';

class Formats {
  Formats._();

  static const standardFormats = [
    plainText,
    htmlText,
    fileUri,
    uri,
    imageJpeg,
    imagePng,
    imageSvg,
    imageGif,
    imageWebP,
    imageTiff,
  ];

  static const plainText = SimpleDataFormat<String>(
    ios: SimplePlatformCodec(
      formats: ['public.utf8-plain-text', 'public.plain-text'],
      onDecode: fromSystemUtf8,
    ),
    macos: SimplePlatformCodec(
      formats: ['public.utf8-plain-text'],
      onDecode: fromSystemUtf8,
    ),
    windows: SimplePlatformCodec(
      formats: [cfUnicodeText],
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
  static const htmlText = SimpleDataFormat<String>(
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
      formats: [cfHdrop],
      onDecode: fileUriFromWindowsPath,
      onEncode: fileUriToWindowsPath,
    ),
    fallback: SimplePlatformCodec<Uri>(
      formats: ['text/uri-list'],
      onDecode: fileUriFromString,
      onEncode: fileUriToString,
    ),
  );

  static const uri = SimpleDataFormat<NamedUri>(
    macos: SimplePlatformCodec(
      decodingFormats: ['public.url', 'public.utf8-plain-text'],
      encodingFormats: [
        'public.url',
        'public.url-name',
        'public.utf8-plain-text'
      ],
      onDecode: macosNamedUriFromSystem,
      onEncode: macosNamedUriToSystem,
    ),
    ios: SimplePlatformCodec(
      formats: ['public.url', 'public.utf8-plain-text'],
      onDecode: iosNamedUriFromSystem,
      onEncode: iosNamedUriToSystem,
    ),
    windows: SimplePlatformCodec(
      decodingFormats: [
        'UniformResourceLocatorW',
        'UniformResourceLocator',
        cfUnicodeText,
      ],
      encodingFormats: [
        'UniformResourceLocatorW',
        cfUnicodeText,
      ],
      onDecode: windowsNamedUriFromSystem,
      onEncode: namedUriToSystem,
    ),
    web: SimplePlatformCodec(
      encodingFormats: [
        'text/plain' // writing uri-list is not supported on web
      ],
      decodingFormats: ['text/uri-list', 'text/plain'],
      onDecode: namedUriFromSystem,
      onEncode: namedUriToSystem,
    ),
    fallback: SimplePlatformCodec(
      formats: ['text/uri-list', 'text/plain'],
      onDecode: namedUriFromSystem,
      onEncode: namedUriToSystem,
    ),
  );

  static const imageJpeg = SimpleDataFormat<Uint8List>(
    macos: SimplePlatformCodec(formats: ['public.jpeg']),
    ios: SimplePlatformCodec(formats: ['public.jpeg']),
    windows: SimplePlatformCodec(formats: ['JFIF']),
    fallback: SimplePlatformCodec(formats: ['image/jpeg']),
  );

  /// PNG Image format
  ///
  /// Note that on Windows, native DIB and DIBV5 image formats will
  /// be also exposed as PNG to the Flutter client (unless there is another
  /// PNG present in the clipboard).
  ///
  /// It also works the other way around: When some other program requests
  /// DIB or DIBV5 image and the clipboard content provided by flutter client
  /// only contains PNG, GIF or JPEG image, the DIB/DIBV5 content will be
  /// automatically generated.
  ///
  /// The conversion in both ways is done on-demand, only when needed.
  /// The provided DIBV5 variant preserves transparency, though in general
  /// support for DIBV5 in Windows applications varies.
  ///
  /// On MacOS, TIFF image in pasteboard will be exposed as PNG unless there
  /// is another PNG already present in the clipboard.
  static const imagePng = SimpleDataFormat<Uint8List>(
    macos: SimplePlatformCodec(formats: ['public.png']),
    ios: SimplePlatformCodec(formats: ['public.png']),
    windows: SimplePlatformCodec(formats: ['PNG']),
    fallback: SimplePlatformCodec(formats: ['image/png']),
  );

  static const imageGif = SimpleDataFormat<Uint8List>(
    macos: SimplePlatformCodec(formats: ['public.gif']),
    ios: SimplePlatformCodec(formats: ['public.gif']),
    windows: SimplePlatformCodec(formats: ['GIF']),
    fallback: SimplePlatformCodec(formats: ['image/gif']),
  );

  static const imageTiff = SimpleDataFormat<Uint8List>(
    macos: SimplePlatformCodec(formats: ['public.tiff']),
    ios: SimplePlatformCodec(formats: ['public.tiff']),
    windows: SimplePlatformCodec(formats: [cfTiff]),
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
}

class NamedUri {
  NamedUri(this.uri, {this.name});

  final Uri uri;

  /// Supported on macOS and iOS, ignored on other platforms.
  String? name;
}

class CustomDataFormat<T extends Object> extends DataFormat<T> {
  final String applicationId;
  final Future<T?> Function(Object value, String platformType)? onDecode;
  final FutureOr<Object> Function(T value, String platformType)? onEncode;

  const CustomDataFormat({
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
