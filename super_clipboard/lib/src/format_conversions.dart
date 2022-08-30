import 'dart:convert';
import 'dart:typed_data';

import 'format.dart';
import 'standard_formats.dart';

// These types will be converted to CF constants with number
// appended to the prefix
const cfInternalPrefix = 'NativeShell_CF_';
const cfUnicodeText = '${cfInternalPrefix}13';
const cfHdrop = '${cfInternalPrefix}15';
const cfTiff = '${cfInternalPrefix}6';

class FormatException implements Exception {
  final String message;
  FormatException(this.message);
}

Future<String?> fromSystemUtf8(
    PlatformDataProvider dataProvider, PlatformFormat format) async {
  final value = await dataProvider.getData(format);
  if (value == null) {
    return null;
  } else if (value is String) {
    return value;
  } else if (value is List<int>) {
    return utf8.decode(value, allowMalformed: true);
  } else if (value is Map && value.isEmpty) {
    // MS office on macOS weirdness when copying empty text with images
    return '';
  } else {
    throw FormatException('Unsupported value type: ${value.runtimeType}');
  }
}

String? _fromSystemUtf16NullTerminated(Object? value) {
  if (value == null) {
    return null;
  } else if (value is String) {
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

Future<String?> fromSystemUtf16NullTerminated(
    PlatformDataProvider dataProvider, PlatformFormat format) async {
  final value = await dataProvider.getData(format);
  return _fromSystemUtf16NullTerminated(value);
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

Future<String?> windowsHtmlFromSystem(
    PlatformDataProvider dataProvider, PlatformFormat format) async {
  final value = await dataProvider.getData(format);
  if (value == null) {
    return null;
  }
  if (format == cfHtml) {
    if (value is List<int>) {
      String decoded = utf8.decode(value, allowMalformed: true);
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
          return utf8.decode(
            value.sublist(startFragment, endFragment),
            allowMalformed: true,
          );
        }
      }
      throw FormatException('Malformed CFHTML');
    }
    throw FormatException('Unsupported value type: ${value.runtimeType}');
  } else {
    return _fromSystemUtf16NullTerminated(value);
  }
}

String fileUriToString(Uri uri, PlatformFormat format) => uri.toString();

Future<Uri?> fileUriFromString(
    PlatformDataProvider dataProvider, PlatformFormat format) async {
  final uri = await fromSystemUtf8(dataProvider, format);
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

Future<Uri?> fileUriFromWindowsPath(
    PlatformDataProvider provider, PlatformFormat format) async {
  final path = await fromSystemUtf16NullTerminated(provider, format);
  if (path is String) {
    return Uri.file(path, windows: true);
  } else {
    return null;
  }
}

Object? macosNamedUriToSystem(NamedUri uri, PlatformFormat format) {
  switch (format) {
    case 'public.url':
    case 'public.utf8-plain-text':
      return uri.uri.toString();
    case 'public.url-name':
      return uri.name;
    default:
      return null;
  }
}

Future<NamedUri?> macosNamedUriFromSystem(
    PlatformDataProvider provider, PlatformFormat format) async {
  // request both futures at the same time
  final futures = await Future.wait([
    fromSystemUtf8(provider, 'public.url-name'),
    fromSystemUtf8(provider, format)
  ]);
  Object? name = futures[0];
  Object? value = futures[1];
  if (value is String) {
    final uri = Uri.tryParse(value);
    if (uri != null) {
      if (format == 'public.utf8-plain-text' && !uri.hasScheme) {
        return null;
      } else {
        return NamedUri(uri,
            name:
                name is String && name.trim().isNotEmpty ? name.trim() : null);
      }
    }
  }
  return null;
}

Object? iosNamedUriToSystem(NamedUri uri, PlatformFormat format) {
  if (format == 'public.utf8-plain-text') {
    return uri.uri.toString();
  } else {
    return [
      uri.uri.toString(),
      '',
      {
        if (uri.name != null) 'title': uri.name,
      }
    ];
  }
}

Future<NamedUri?> iosNamedUriFromSystem(
    PlatformDataProvider provider, PlatformFormat format) async {
  final Object? value = await provider.getData(format);
  if (value is Uint8List) {
    final uri = Uri.tryParse(utf8.decode(value, allowMalformed: true));
    if (uri != null) {
      if (format == 'public.utf8-plain-text' && !uri.hasScheme) {
        return null;
      }
      return NamedUri(uri);
    } else {
      return null;
    }
  } else if (value is List && value.isNotEmpty && value[0] is String) {
    final uri = Uri.tryParse(value[0]);
    String? name;
    if (value.length >= 3 && value[2] is String) {
      name = value[2]['title'];
    }
    if (uri != null) {
      return NamedUri(uri, name: name is String ? name : null);
    }
  }
  return null;
}

Future<NamedUri?> windowsNamedUriFromSystem(
    PlatformDataProvider provider, PlatformFormat format) async {
  String? value;
  if (format == 'UniformResourceLocator') {
    // It is really ANSI but we try to decode as UTF8
    value = await fromSystemUtf8(provider, format);
  } else {
    value = await fromSystemUtf16NullTerminated(provider, format);
  }
  if (value is String) {
    final uri = Uri.tryParse(value);
    if (uri != null) {
      // If we're parsing plain text insist on URL with scheme
      if (format == cfUnicodeText && !uri.hasScheme) {
        return null;
      }
      return NamedUri(uri);
    }
  }
  return null;
}

Future<NamedUri?> namedUriFromSystem(
    PlatformDataProvider provider, PlatformFormat format) async {
  final value = await fromSystemUtf8(provider, format);
  if (value is String) {
    final uri = Uri.tryParse(value);
    if (uri != null) {
      if (format == 'text/plain' && !uri.hasScheme) {
        return null;
      }
      return NamedUri(uri);
    }
    return null;
  } else {
    return null;
  }
}

Object namedUriToSystem(NamedUri uri, PlatformFormat format) {
  return uri.uri.toString();
}
