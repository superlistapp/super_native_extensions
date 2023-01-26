import 'dart:async';

import 'format.dart';
import 'format_conversions.dart';
import 'formats_base.dart';
import 'platform.dart';

class Formats {
  Formats._();

  static const List<DataFormat> standardFormats = [
    plainText,
    htmlText,
    fileUri,
    uri,
    jpeg,
    png,
    svg,
    gif,
    webp,
    tiff,
    utf8Text,
    webUnknown,
  ];

  static const plainText = SimpleValueFormat<String>(
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
  static const htmlText = SimpleValueFormat<String>(
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

  static const fileUri = SimpleValueFormat<Uri>(
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

  static const uri = SimpleValueFormat<NamedUri>(
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
      // writing uri-list to clipboard is not supported on web
      // and it will be silently skipped
      formats: ['text/uri-list', 'text/plain'],
      onDecode: namedUriFromSystem,
      onEncode: namedUriToSystem,
    ),
    fallback: SimplePlatformCodec(
      formats: ['text/uri-list', 'text/plain'],
      onDecode: namedUriFromSystem,
      onEncode: namedUriToSystem,
    ),
  );

  static const jpeg = SimpleFileFormat(
    macosFormats: ['public.jpeg'],
    iosFormats: ['public.jpeg'],
    windowsFormats: ['JFIF'],
    fallbackFormats: ['image/jpeg'],
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
  static const png = SimpleFileFormat(
    macosFormats: ['public.png'],
    iosFormats: ['public.png'],
    windowsFormats: ['PNG'],
    fallbackFormats: ['image/png'],
  );

  static const gif = SimpleFileFormat(
    macosFormats: ['public.gif'],
    iosFormats: ['public.gif'],
    windowsFormats: ['GIF'],
    fallbackFormats: ['image/gif'],
  );

  static const tiff = SimpleFileFormat(
    macosFormats: ['public.tiff'],
    iosFormats: ['public.tiff'],
    windowsFormats: [cfTiff],
    fallbackFormats: ['image/tiff'],
  );

  static const webp = SimpleFileFormat(
    macosFormats: ['org.webmproject.webp'],
    iosFormats: ['org.webmproject.webp'],
    fallbackFormats: ['image/webp'],
  );

  static const svg = SimpleFileFormat(
    macosFormats: ['public.svg-image'],
    iosFormats: ['public.svg-image'],
    fallbackFormats: ['image/svg+xml'],
  );

  /// Format to be used for UTF-8 encoded files. Like other file format, this
  /// does no conversion.
  static const utf8Text = SimpleFileFormat(
    iosFormats: ['public.utf8-plain-text', 'public.plain-text'],
    macosFormats: ['public.utf8-plain-text', 'public.plain-text'],
    fallbackFormats: ['text/plain'],
  );

  /// Some browsers (Safari, of course, who else), do not report mime types of
  /// files during dragging, only when dropped. In which case there will be one
  /// item present during the drop over event of type [webUnknown].
  static const webUnknown = SimpleFileFormat(
    webFormats: ['web:unknown'],
    fallbackFormats: ['text/plain'],
  );
}

class NamedUri {
  NamedUri(this.uri, {this.name});

  final Uri uri;

  /// Supported on macOS and iOS, ignored on other platforms.
  String? name;
}

class CustomValueFormat<T extends Object> extends ValueFormat<T> {
  final String applicationId;
  final Future<T?> Function(Object value, String platformType)? onDecode;
  final FutureOr<Object> Function(T value, String platformType)? onEncode;

  const CustomValueFormat({
    required this.applicationId,
    this.onDecode,
    this.onEncode,
  });

  @override
  PlatformCodec<T> get codec => _codecForPlatform(currentPlatform);

  PlatformCodec<T> _codecForPlatform(ClipboardPlatform platform) {
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
