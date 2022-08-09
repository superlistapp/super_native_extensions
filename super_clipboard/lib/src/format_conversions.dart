import 'dart:convert';
import 'dart:typed_data';

import 'format.dart';

class FormatException implements Exception {
  final String message;
  FormatException(this.message);
}

String fromSystemUtf8(Object value, PlatformFormat format) {
  if (value is String) {
    return value;
  } else if (value is List<int>) {
    return utf8.decode(value);
  } else {
    throw FormatException('Unsupported value type: ${value.runtimeType}');
  }
}

String fromSystemUtf16NullTerminated(Object value, PlatformFormat format) {
  if (value is String) {
    return value;
  } else if (value is TypedData) {
    var codeUnits = value.buffer
        .asUint16List(value.offsetInBytes, value.lengthInBytes ~/ 2);
    while (codeUnits.isNotEmpty && codeUnits.last == 0) {
      codeUnits = codeUnits.sublist(0, codeUnits.length - 1);
    }
    return String.fromCharCodes(codeUnits);
  } else {
    throw FormatException('Unsupported value type: ${value.runtimeType}');
  }
}

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

Object windowsHtmlToSystem(String text, PlatformFormat format) {
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

String windowsHtmlFromSystem(Object value, PlatformFormat format) {
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
      throw FormatException('Malformed CFHTML');
    }
    throw FormatException('Unsupported value type: ${value.runtimeType}');
  } else {
    return fromSystemUtf16NullTerminated(value, format);
  }
}

String fileUriToString(Uri uri, PlatformFormat format) => uri.toString();

Uri? fileUriFromString(Object uri, PlatformFormat format) {
  if (uri is String) {
    final res = Uri.tryParse(uri);
    if (res?.isScheme('file') == true) {
      return res;
    }
  }
  return null;
}

String fileUriToWindowsPath(Uri uri, PlatformFormat format) =>
    uri.toFilePath(windows: true);

Uri? fileUriFromWindowsPath(Object path, PlatformFormat format) {
  if (path is String) {
    return Uri.file(path, windows: true);
  } else {
    return null;
  }
}
