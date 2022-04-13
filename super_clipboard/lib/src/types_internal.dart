import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'common.dart';

String? fromSystemUtf8(Object value, String type) {
  if (value is String) {
    return value;
  } else if (value is List<int>) {
    return utf8.decode(value);
  } else {
    return null;
  }
}

String? fromSystemUtf16NullTerminated(Object value, String format) {
  if (value is String) {
    return value;
  } else if (value is List<int>) {
    var codeUnits = value; // need extra variable for flutter type inference
    while (codeUnits.isNotEmpty && codeUnits.last == 0) {
      codeUnits = codeUnits.sublist(0, codeUnits.length - 1);
    }
    return String.fromCharCodes(codeUnits);
  } else {
    return null;
  }
}

// Platform plugin will try to coerce String to expected type
Object passthrough(dynamic value, String type) => value;

class SimpleClipboardType<T> extends ClipboardType<T> {
  final ClipboardPlatformType<T>? android;
  final ClipboardPlatformType<T>? ios;
  final ClipboardPlatformType<T>? linux;
  final ClipboardPlatformType<T>? macos;
  final ClipboardPlatformType<T>? windows;

  const SimpleClipboardType({
    this.android,
    this.ios,
    this.linux,
    this.macos,
    this.windows,
  });

  @override
  ClipboardPlatformType<T> platformTypeFor(ClipboardPlatform platform) {
    switch (platform) {
      case ClipboardPlatform.android:
        return android ?? const FallbackClipboardPlatformType();
      case ClipboardPlatform.ios:
        return ios ?? const FallbackClipboardPlatformType();
      case ClipboardPlatform.linux:
        return linux ?? const FallbackClipboardPlatformType();
      case ClipboardPlatform.macos:
        return macos ?? const FallbackClipboardPlatformType();
      case ClipboardPlatform.windows:
        return windows ?? const FallbackClipboardPlatformType();
    }
  }
}

class FallbackClipboardPlatformType<T> extends ClipboardPlatformType<T> {
  const FallbackClipboardPlatformType();

  @override
  Future<T> convertFromSystem(Object value, String platformType) {
    throw UnimplementedError();
  }

  @override
  Future<Object> convertToSystem(T value, String platformType) {
    throw UnimplementedError();
  }

  @override
  List<String> readableSystemTypes() => [];

  @override
  List<String> writableSystemTypes() => [];
}

class SimpleClipboardPlatformType<T> extends ClipboardPlatformType<T> {
  const SimpleClipboardPlatformType({
    required this.onConvertFromSystem,
    required this.onConvertToSystem,
    required this.types,
  });

  final FutureOr<T?> Function(Object value, String platformType)
      onConvertFromSystem;
  final FutureOr<Object> Function(T value, String platformType)
      onConvertToSystem;
  final List<String> types;

  @override
  FutureOr<T?> convertFromSystem(Object value, String platformType) =>
      onConvertFromSystem(value, platformType);

  @override
  FutureOr<Object> convertToSystem(value, String platformType) =>
      onConvertToSystem(value, platformType);

  @override
  List<String> readableSystemTypes() => types;

  @override
  List<String> writableSystemTypes() => types;
}

// These types will be converted to CF constants with number
// appended to the prefix
const cfInternalPrefix = 'NativeShell_InternalWindowsFormat_';

// https://docs.microsoft.com/en-us/windows/win32/dataxchg/html-clipboard-format
// https://docs.microsoft.com/en-us/troubleshoot/developer/visualstudio/cpp/general/add-html-code-clipboard
const cfHtml = 'HTML Format';

Uint8List _createHeader({
  int startHtml = 0,
  int endHtml = 0,
  int startFragment = 0,
  int endFragment = 0,
  required bool includeHtml,
}) {
  String format(int number) {
    return number.toString().padLeft(8, '0');
  }

  const le = '\r\n';
  final buffer = StringBuffer();
  buffer.write("Version:0.9$le");
  buffer.write("StartHTML:${format(startHtml)}$le");
  buffer.write("EndHTML:${format(endHtml)}$le");
  buffer.write("StartFragment:${format(startFragment)}$le");
  buffer.write("EndFragment:${format(endFragment)}$le");
  if (includeHtml) {
    buffer.write("<html><body>$le");
    buffer.write("<!--StartFragment -->");
  }
  return utf8.encode(buffer.toString()) as Uint8List;
}

Uint8List _createFooter() {
  return utf8.encode('<!--EndFragment-->\r\n</body>\r\n</html>') as Uint8List;
}

Object windowsHtmlToSystem(String text, String format) {
  if (format == cfHtml) {
    final headerLength = _createHeader(includeHtml: true).length;
    final lines = const LineSplitter().convert(text);
    final textEncoded = utf8.encode(lines.join('\r\n')) as Uint8List;
    final footer = _createFooter();
    final totalLength = headerLength + textEncoded.length + footer.length;
    final header = _createHeader(
      startHtml: _createHeader(
        includeHtml: false,
      ).length,
      startFragment: headerLength,
      endFragment: headerLength + textEncoded.length,
      endHtml: totalLength,
      includeHtml: true,
    );

    final res =
        Uint8List(headerLength + textEncoded.length + footer.length + 1);
    res.setAll(0, header);
    res.setAll(header.length, textEncoded);
    res.setAll(header.length + textEncoded.length, footer);
    res[res.length - 1] = 0; // null termination
    return res;
  } else {
    return text;
  }
}

String? windowsHtmlFromSystem(Object value, String format) {
  if (format == cfHtml) {
    if (value is List<int>) {
      String decoded = utf8.decode(value);
      final lines = const LineSplitter().convert(decoded);
      const startFragmentPrefix = 'StartFragment:';
      const endFragmentPrefix = 'EndFragment:';
      int startFragment = -1;
      int endFragment = -1;
      for (final line in lines) {
        if (line.startsWith(startFragmentPrefix)) {
          startFragment = int.parse(line.substring(startFragmentPrefix.length));
        }
        if (line.startsWith(endFragmentPrefix)) {
          endFragment = int.parse(line.substring(endFragmentPrefix.length));
        }
        if (startFragment != -1 && endFragment != -1) {
          return utf8.decode(value.sublist(startFragment, endFragment));
        }
      }
    }
    return null;
  } else {
    return fromSystemUtf16NullTerminated(value, format);
  }
}

