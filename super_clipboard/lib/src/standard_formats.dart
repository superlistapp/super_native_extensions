import 'dart:async';

import 'format.dart';
import 'format_conversions.dart';
import 'formats_base.dart';
import 'platform.dart';

class Formats {
  Formats._();

  static const List<DataFormat> standardFormats = [
    //
    // Value formats
    //
    plainText,
    htmlText,
    uri,
    fileUri,

    //
    // File formats
    //
    plainTextFile,
    htmlFile,
    jpeg,
    png,
    svg,
    gif,
    webp,
    tiff,
    bmp,
    ico,
    heic,
    heif,
    mp4,
    mov,
    m4v,
    avi,
    mpeg,
    webm,
    ogg,
    wmv,
    flv,
    mkv,
    ts,
    mp3,
    oga,
    aac,
    wav,
    pdf,
    doc,
    docx,
    csv,
    xls,
    xlsx,
    ppt,
    pptx,
    rtf,
    json,
    zip,
    tar,
    gzip,
    bzip2,
    xz,
    rar,
    jar,
    sevenZip,
    dmg,
    iso,
    deb,
    rpm,
    apk,
    exe,
    msi,
    dll,
    webUnknown,
  ];

  /// Value format for plain text. This is used for copying and pasting text
  /// as well as dragging and dropping plain text snippets. This format takes
  /// care of conversion from/to platform specific encoding.
  ///
  /// When consuming dropped text files use [plainTextFile] format instead.
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

  /// Key for rich text in form of html snippet. Make sure to include [plainText]
  /// version in clipboard as well, otherwise setting the content may fail on some
  /// platforms (i.e. Android).
  //.
  /// Note that if you wish to receive dropped HTML files use [htmlFile] format
  /// instead. `htmlText` is mostly meant for copying and pasting HTML snippets
  /// (which on some platforms require additional conversion).
  static const htmlText = SimpleValueFormat<String>(
    ios: SimplePlatformCodec<String>(
      formats: ['public.html'],
      onDecode: fromSystemUtf8,
      onEncode: htmlToSystem,
    ),
    macos: SimplePlatformCodec<String>(
      formats: ['public.html'],
      onDecode: fromSystemUtf8,
      onEncode: htmlToSystem,
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

  //
  // File Formats
  //

  //
  // Image
  //

  static const jpeg = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.jpeg'],
    windowsFormats: ['JFIF'],
    mimeTypes: ['image/jpeg'],
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
    uniformTypeIdentifiers: ['public.png'],
    windowsFormats: ['PNG'],
    mimeTypes: ['image/png'],
  );

  static const gif = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.compuserve.gif'],
    windowsFormats: ['GIF'],
    mimeTypes: ['image/gif'],
  );

  static const tiff = SimpleFileFormat(
    macosFormats: ['public.tiff'],
    iosFormats: ['public.tiff'],
    windowsFormats: [cfTiff],
    fallbackFormats: ['image/tiff'],
  );

  static const webp = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.webmproject.webp'],
    mimeTypes: ['image/webp'],
  );

  static const svg = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.svg-image'],
    mimeTypes: ['public.svg-image'],
  );

  static const bmp = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.microsoft.bmp'],
    mimeTypes: ['image/bmp'],
  );

  static const ico = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.microsoft.ico'],
    mimeTypes: ['image/x-icon'],
  );

  static const heic = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.heic'],
    mimeTypes: ['image/heic'],
  );

  static const heif = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.heif'],
    mimeTypes: ['image/heif'],
  );

  //
  // Video
  //

  static const mp4 = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.mpeg-4'],
    mimeTypes: ['video/mp4'],
  );

  static const mov = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.apple.quicktime-movie'],
    mimeTypes: ['video/quicktime'],
  );

  static const m4v = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.apple.m4v-video'],
    mimeTypes: ['video/x-m4v'],
  );

  static const avi = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.avi'],
    mimeTypes: ['video/x-msvideo'],
  );

  static const mpeg = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.mpeg'],
    mimeTypes: ['video/mpeg'],
  );

  static const webm = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.webmproject.webm'],
    mimeTypes: ['video/webm'],
  );

  static const ogg = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.xiph.ogg.theora'],
    mimeTypes: ['video/ogg'],
  );

  static const wmv = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.microsoft.wmv'],
    mimeTypes: ['vvideo/x-ms-wmv'],
  );

  static const flv = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.adobe.flash-video'],
    mimeTypes: ['video/x-flv'],
  );

  static const mkv = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.matroska.mkv'],
    mimeTypes: ['video/x-matroska'],
  );

  static const ts = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.apple.mpeg-2-transport-stream'],
    mimeTypes: ['video/vnd.dlna.mpeg-tts'],
  );

  //
  // Audio
  //

  static const mp3 = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.mp3'],
    mimeTypes: ['audio/mpeg'],
  );

  static const m4a = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.mpeg-4-audio'],
    mimeTypes: ['audio/mp4'],
  );

  static const oga = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.xiph.ogg.vorbis'],
    mimeTypes: ['audio/ogg'],
  );

  static const aac = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.aac-audio'],
    mimeTypes: ['audio/aac'],
  );

  static const wav = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.wav'],
    mimeTypes: ['audio/wav'],
  );

  static const opus = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.xiph.opus'],
    mimeTypes: ['audio/ogg'],
  );

  static const flac = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.xiph.flac'],
    mimeTypes: ['audio/flac'],
  );

  //
  // Document
  //

  static const pdf = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.adobe.pdf'],
    mimeTypes: ['application/pdf'],
  );

  static const doc = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.microsoft.word.doc'],
    mimeTypes: ['application/msword'],
  );

  static const docx = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.openxmlformats.wordprocessingml.document'],
    mimeTypes: [
      'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
    ],
  );

  static const epub = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.idpf.epub-container'],
    mimeTypes: ['application/epub+zip'],
  );

  static const md = SimpleFileFormat(
    uniformTypeIdentifiers: ['net.daringfireball.markdown'],
    mimeTypes: ['text/markdown'],
  );

  static const csv = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.comma-separated-values-text'],
    mimeTypes: ['text/csv'],
  );

  static const xls = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.microsoft.excel.xls'],
    mimeTypes: ['application/vnd.ms-excel'],
  );

  static const xlsx = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.openxmlformats.spreadsheetml.sheet'],
    mimeTypes: [
      'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
    ],
  );

  static const ppt = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.microsoft.powerpoint.​ppt'],
    mimeTypes: ['application/vnd.ms-powerpoint'],
  );

  static const pptx = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.openxmlformats.presentationml.presentation'],
    mimeTypes: [
      'application/vnd.openxmlformats-officedocument.presentationml.presentation'
    ],
  );

  static const rtf = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.rtf'],
    mimeTypes: ['application/rtf'],
  );

  static const json = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.json'],
    mimeTypes: ['application/json'],
  );

  //
  // Archive
  //

  static const zip = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.zip-archive'],
    mimeTypes: ['application/zip'],
  );

  static const tar = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.tar-archive'],
    mimeTypes: ['application/x-tar'],
  );

  static const gzip = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.gzip'],
    mimeTypes: ['application/gzip'],
  );

  static const bzip2 = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.bzip2-archive'],
    mimeTypes: ['application/x-bzip2'],
  );

  static const xz = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.xz-archive'],
    mimeTypes: ['application/x-xz'],
  );

  static const rar = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.rarlab.rar'],
    mimeTypes: ['application/x-rar-compressed'],
  );

  static const jar = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.sun.java-archive'],
    mimeTypes: ['application/java-archive'],
  );

  static const sevenZip = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.7-zip.7-zip-archive'],
    mimeTypes: ['application/x-7z-compressed'],
  );

  static const dmg = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.apple.disk-image-udif'],
    mimeTypes: ['application/x-apple-diskimage'],
  );

  static const iso = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.iso-image'],
    mimeTypes: ['application/x-iso9660-image'],
  );

  static const deb = SimpleFileFormat(
    uniformTypeIdentifiers: ['org.debian.deb-archive'],
    mimeTypes: ['application/x-debian-package'],
  );

  static const rpm = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.redhat.rpm-archive'],
    mimeTypes: ['application/x-rpm'],
  );

  static const apk = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.android.package-archive'],
    mimeTypes: ['application/vnd.android.package-archive'],
  );

  // Executable

  static const exe = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.microsoft.windows-executable'],
    mimeTypes: ['application/x-msdownload'],
  );

  static const msi = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.microsoft.msi-installer'],
    mimeTypes: ['application/x-msi'],
  );

  static const dll = SimpleFileFormat(
    uniformTypeIdentifiers: ['com.microsoft.windows-dynamic-link-library'],
    mimeTypes: ['application/x-msdownload'],
  );

  /// Used when dropping a plain text file. Client is responsible for dealing
  /// with file encoding.
  static const plainTextFile = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.utf8-plain-text', 'public.plain-text'],
    mimeTypes: ['text/plain'],
  );

  /// Used when dropping a HTML file (not a HTML snippet). Client is responsible for dealing
  /// with file encoding.
  static const htmlFile = SimpleFileFormat(
    uniformTypeIdentifiers: ['public.html'],
    mimeTypes: ['text/html'],
  );

  /// Deprecated. Original name was misleading because the UTF8 encoding is not
  /// enforced.
  @Deprecated('Use plainTextFile instead.')
  static const utf8Text = plainTextFile;

  /// Some browsers (Safari, of course, who else), do not report mime types of
  /// files during dragging, only when dropped. In which case there will be one
  /// item present during the drop over event of type [webUnknown].
  static const webUnknown = SimpleFileFormat(
    webFormats: ['web:unknown'],
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
