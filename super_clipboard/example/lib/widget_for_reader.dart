import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:collection/collection.dart';
import 'package:super_clipboard/super_clipboard.dart';

const formatCustom = CustomValueFormat<Uint8List>(
  applicationId: "com.superlist.clipboard.Example.CustomType",
);

void buildWidgetsForReaders(
  BuildContext context,
  Iterable<ReaderInfo> readers,
  ValueChanged<List<Widget>> onWidgets,
) {
  final widgets = Future.wait(
    readers.mapIndexed(
      (index, element) => _buildWidgetForReader(context, element, index),
    ),
  );
  // Instead of await invoke callback when widgets are built.
  widgets.then((value) => onWidgets(value));
}

class ReaderInfo {
  ReaderInfo._({
    required this.reader,
    required this.suggestedName,
    required List<_PlatformFormat> formats,
    this.localData,
  }) : _formats = formats;

  static Future<ReaderInfo> fromReader(
    DataReader reader, {
    Object? localData,
  }) async {
    // build list of native formats with virtual/synthetized flags
    final List<String> formats = reader.platformFormats;

    final List<String> rawFormats =
        await reader.rawReader!.getAvailableFormats();

    // Reader may synthetize format from URI.
    List<String> synthetizedByReader = List.of(formats)
      ..removeWhere((element) => rawFormats.contains(element));

    final virtual =
        await Future.wait(formats.map((e) => reader.rawReader!.isVirtual(e)));

    final synthetized = await Future.wait(formats.map((e) async =>
        await reader.rawReader!.isSynthetized(e) ||
        synthetizedByReader.contains(e)));

    return ReaderInfo._(
      reader: reader,
      suggestedName: await reader.getSuggestedName(),
      localData: localData,
      formats: formats
          .mapIndexed((index, element) => _PlatformFormat(
                element,
                virtual: virtual[index],
                synthetized: synthetized[index],
              ))
          .toList(growable: false),
    );
  }

  final DataReader reader;
  final String? suggestedName;
  final List<_PlatformFormat> _formats;
  final Object? localData;
}

class _PlatformFormat {
  final PlatformFormat format;
  final bool virtual;
  final bool synthetized;

  _PlatformFormat(
    this.format, {
    required this.virtual,
    required this.synthetized,
  });
}

/// Turn [DataReader.getValue] into a future.
extension _ReadValue on DataReader {
  Future<T?> readValue<T extends Object>(DataFormat<T> format) {
    final c = Completer<T?>();
    getValue<T>(format, (value) {
      if (value.error != null) {
        c.completeError(value.error!);
      } else {
        c.complete(value.value);
      }
    });
    return c.future;
  }

  Future<Uint8List?>? readFile(FileFormat format) {
    final c = Completer<Uint8List?>();
    getValue<Uint8List>(format, (value) async {
      if (value.error != null) {
        c.completeError(value.error!);
      } else {
        try {
          final all = await value.readAll();
          c.complete(all);
        } catch (e) {
          c.completeError(e);
        }
      }
    });
    return c.future;
  }
}

/// Builds widget containing information for data reader.
Future<Widget> _buildWidgetForReader(
  BuildContext context,
  ReaderInfo reader,
  int index,
) async {
  final itemFormats = reader.reader.getFormats([
    ...Formats.standardFormats,
    formatCustom,
  ]);

  // Request all data before awaiting
  final futures =
      itemFormats.map((e) => _widgetForFormat(context, e, reader.reader));

  // Now await all futures
  final widgets = await Future.wait(futures);
  final children = widgets
      .where((element) => element != null)
      .cast<_RepresentationWidget>()
      .toList(growable: true);

  // remove duplicate widgets
  final formats = <DataFormat>{};
  children.retainWhere((element) => formats.add(element.format));

  // build list of native formats with virtual/synthetized flags
  final nativeFormats = reader._formats.map((e) {
    final attributes = [
      if (e.virtual) 'virtual',
      if (e.synthetized) 'synthetized',
    ].join(', ');
    return attributes.isNotEmpty ? '${e.format} ($attributes)' : e.format;
  }).toList(growable: false);

  return _ReaderWidget(
    itemName: 'Data item $index',
    suggestedFileName: reader.suggestedName ?? 'null',
    representations: children,
    nativeFormats: nativeFormats,
  );
}

class _ReaderWidget extends StatelessWidget {
  const _ReaderWidget({
    required this.itemName,
    required this.suggestedFileName,
    required this.representations,
    required this.nativeFormats,
  });

  final String itemName;
  final String suggestedFileName;
  final List<Widget> representations;
  final List<String>? nativeFormats;

  @override
  Widget build(BuildContext context) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _HeaderWidget(
              itemName: itemName, suggestedFileName: suggestedFileName),
          ...representations.intersperse(const SizedBox(height: 2)),
          if (nativeFormats != null) ...[
            const SizedBox(height: 2),
            _FooterWidget(nativeFormats: nativeFormats!),
          ]
        ],
      ),
    );
  }
}

class _HeaderWidget extends StatelessWidget {
  const _HeaderWidget({
    required this.itemName,
    required this.suggestedFileName,
  });

  final String itemName;
  final String suggestedFileName;

  @override
  Widget build(BuildContext context) {
    return Container(
      color: Colors.blueGrey.shade100,
      padding: const EdgeInsets.all(10),
      child: Row(
        children: [
          Text(
            itemName,
            style: const TextStyle(fontWeight: FontWeight.bold),
          ),
          const SizedBox(width: 14),
          Flexible(
            child: Text('(Suggested file name: $suggestedFileName)',
                style: TextStyle(
                  fontSize: 11,
                  color: Colors.grey.shade600,
                )),
          ),
        ],
      ),
    );
  }
}

class _FooterWidget extends StatelessWidget {
  const _FooterWidget({
    required this.nativeFormats,
  });

  @override
  Widget build(BuildContext context) {
    final formats = nativeFormats.join(', ');
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      color: Colors.blueGrey.shade50,
      child: Text.rich(
        TextSpan(
          children: [
            const TextSpan(
              text: 'Native formats: ',
              style: TextStyle(fontWeight: FontWeight.bold),
            ),
            TextSpan(text: formats),
          ],
        ),
        style: TextStyle(fontSize: 11.0, color: Colors.grey.shade600),
      ),
    );
  }

  final List<String> nativeFormats;
}

class _UriWidget extends StatelessWidget {
  const _UriWidget({
    required this.uri,
  });

  final NamedUri uri;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(uri.uri.toString()),
        if (uri.name != null)
          DefaultTextStyle.merge(
            style: TextStyle(color: Colors.grey.shade600),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text('Name: '),
                Expanded(
                  child: Text(uri.name!),
                ),
              ],
            ),
          )
      ],
    );
  }
}

class _RepresentationWidget extends StatelessWidget {
  const _RepresentationWidget({
    required this.format,
    required this.name,
    required this.synthetized,
    required this.virtual,
    required this.content,
  });

  @override
  Widget build(BuildContext context) {
    final tag = [
      if (virtual) 'virtual',
      if (synthetized) 'synthetized',
    ].join(' ');
    return DefaultTextStyle.merge(
      style: const TextStyle(fontSize: 12),
      child: Container(
        decoration: BoxDecoration(
          color: Colors.blueGrey.shade50,
        ),
        padding: const EdgeInsets.symmetric(horizontal: 12.0, vertical: 6.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Text(
                  name,
                  style: const TextStyle(fontWeight: FontWeight.bold),
                ),
                Text(tag.isNotEmpty ? ' ($tag)' : ''),
              ],
            ),
            const SizedBox(
              height: 2,
            ),
            content,
          ],
        ),
      ),
    );
  }

  final DataFormat format;
  final String name;
  final bool synthetized;
  final bool virtual;
  final Widget content;
}

Future<_RepresentationWidget?> _widgetForImage(
  BuildContext context,
  FileFormat format,
  String name,
  DataReader reader,
) async {
  final scale = MediaQuery.of(context).devicePixelRatio;
  final image = await reader.readFile(format);
  if (image == null || image.isEmpty /* Tiff on Firefox/Linux */) {
    return null;
  } else {
    return _RepresentationWidget(
      format: format,
      name: 'Image ($name)',
      synthetized: reader.isSynthetized(format),
      virtual: reader.isVirtual(format),
      content: Container(
        padding: const EdgeInsets.only(top: 4),
        alignment: Alignment.centerLeft,
        child: Image.memory(
          image,
          scale: scale,
        ),
      ),
    );
  }
}

Future<_RepresentationWidget?> _widgetForFormat(
    BuildContext context, DataFormat format, DataReader reader) async {
  switch (format) {
    case Formats.plainText:
      final text = await reader.readValue(Formats.plainText);
      if (text == null) {
        return null;
      } else {
        // Sometimes macOS uses CR for line break;
        final sanitized = text.replaceAll(RegExp('\r[\n]?'), '\n');
        return _RepresentationWidget(
          format: format,
          name: 'Plain Text',
          synthetized: reader.isSynthetized(format),
          virtual: reader.isVirtual(format),
          content: Text(sanitized),
        );
      }
    case Formats.utf8Text:
      if (!reader.isVirtual(format) && !reader.isSynthetized(format)) {
        return null;
      }
      final contents = await reader.readFile(Formats.utf8Text);
      if (contents == null) {
        return null;
      } else {
        final text = utf8.decode(contents);
        return _RepresentationWidget(
          format: format,
          name: 'Plain Text (utf8 file)',
          synthetized: reader.isSynthetized(format),
          virtual: reader.isVirtual(format),
          content: Text(text),
        );
      }
    case Formats.htmlText:
      final html = await reader.readValue(Formats.htmlText);
      if (html == null) {
        return null;
      } else {
        return _RepresentationWidget(
          format: format,
          name: 'HTML Text',
          synthetized: reader.isSynthetized(format),
          virtual: reader.isVirtual(format),
          content: Text(html),
        );
      }
    case Formats.png:
      return _widgetForImage(context, Formats.png, 'PNG', reader);
    case Formats.jpeg:
      return _widgetForImage(context, Formats.jpeg, 'JPEG', reader);
    case Formats.gif:
      return _widgetForImage(context, Formats.gif, 'GIF', reader);
    case Formats.tiff:
      return _widgetForImage(context, Formats.tiff, 'TIFF', reader);
    case Formats.webp:
      return _widgetForImage(context, Formats.webp, 'WebP', reader);
    // regular and file uri may have same mime types on some platforms
    case Formats.uri:
    case Formats.fileUri:
      // Make sure to request both values before awaiting
      final fileUriFuture = reader.readValue(Formats.fileUri);
      final uriFuture = reader.readValue(Formats.uri);

      // try file first and if it fails try regular URI
      final fileUri = await fileUriFuture;
      if (fileUri != null) {
        return _RepresentationWidget(
          format: Formats.fileUri,
          name: 'File URI',
          synthetized: reader.isSynthetized(format),
          virtual: reader.isVirtual(format),
          content: Text(fileUri.toString()),
        );
      }
      final uri = await uriFuture;
      if (uri != null) {
        return _RepresentationWidget(
          format: Formats.uri,
          name: 'URI',
          synthetized: reader.isSynthetized(Formats.uri),
          virtual: reader.isVirtual(Formats.uri),
          content: _UriWidget(uri: uri),
        );
      }
      return null;
    case formatCustom:
      final data = await reader.readValue(formatCustom);
      if (data == null) {
        return null;
      } else {
        return _RepresentationWidget(
          format: format,
          name: 'Custom Data',
          synthetized: reader.isSynthetized(formatCustom),
          virtual: reader.isVirtual(formatCustom),
          content: Text(data.toString()),
        );
      }
    default:
      return null;
  }
}

extension IntersperseExtensions<T> on Iterable<T> {
  Iterable<T> intersperse(T element) sync* {
    final iterator = this.iterator;
    if (iterator.moveNext()) {
      yield iterator.current;
      while (iterator.moveNext()) {
        yield element;
        yield iterator.current;
      }
    }
  }
}
