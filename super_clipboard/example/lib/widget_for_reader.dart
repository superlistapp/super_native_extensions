import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:collection/collection.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_clipboard_example/main.dart';

/// Builds widget containing information for data reader.
Future<Widget> buildWidgetForReader(
    BuildContext context, DataReader reader, int index) async {
  final itemFormats = reader.getFormats([
    ...Format.standardFormats,
    formatCustom,
  ]);

  // Request all data before awaiting
  final futures = itemFormats.map((e) => _widgetForFormat(context, e, reader));

  // Now await all futures
  final widgets = await Future.wait(futures);
  final children = widgets
      .where((element) => element != null)
      .cast<_RepresentationWidget>()
      .toList(growable: true);

  // remove duplicate widgets
  final formats = <DataFormat>{};
  children.retainWhere((element) => formats.add(element.format));

  // build list of native formats with virtua/synthetized flags
  List<String>? nativeFormats = await reader.rawReader?.getAvailableFormats();
  if (nativeFormats != null) {
    final virtual = await Future.wait(
        nativeFormats.map((e) => reader.rawReader!.isVirtual(e)));
    final synthetized = await Future.wait(
        nativeFormats.map((e) => reader.rawReader!.isSynthetized(e)));
    nativeFormats = nativeFormats.mapIndexed((i, e) {
      final attributes = [
        if (virtual[i]) 'virtual',
        if (synthetized[i]) 'synthetized',
      ].join(', ');
      return attributes.isNotEmpty ? '$e ($attributes)' : e;
    }).toList(growable: false);
  }

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
      child: Text(
        'Native formats: $formats',
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
    required this.content,
  });

  @override
  Widget build(BuildContext context) {
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
                Text(synthetized ? ' (synthetized)' : ''),
              ],
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
  final Widget content;
}

Future<_RepresentationWidget?> _widgetForImage(
  BuildContext context,
  DataFormat<Uint8List> format,
  String name,
  DataReader reader,
) async {
  final scale = MediaQuery.of(context).devicePixelRatio;
  final image = await reader.readValue(format);
  if (image == null || image.isEmpty /* Tiff on Firefox/Linux */) {
    return null;
  } else {
    return _RepresentationWidget(
      format: format,
      name: 'Image ($name)',
      synthetized: reader.isSynthetized(format),
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
    case Format.plainText:
      final text = await reader.readValue(Format.plainText);
      if (text == null) {
        return null;
      } else {
        // Sometimes macOS uses CR for line break;
        final sanitized = text.replaceAll(RegExp('\r[\n]?'), '\n');
        return _RepresentationWidget(
          format: format,
          name: 'Plain Text',
          synthetized: reader.isSynthetized(Format.plainText),
          content: Text(sanitized),
        );
      }
    case Format.htmlText:
      final html = await reader.readValue(Format.htmlText);
      if (html == null) {
        return null;
      } else {
        return _RepresentationWidget(
          format: format,
          name: 'HTML Text',
          synthetized: reader.isSynthetized(Format.htmlText),
          content: Text(html),
        );
      }
    case Format.imagePng:
      return _widgetForImage(context, Format.imagePng, 'PNG', reader);
    case Format.imageJpeg:
      return _widgetForImage(context, Format.imageJpeg, 'JPEG', reader);
    case Format.imageGif:
      return _widgetForImage(context, Format.imageGif, 'GIF', reader);
    case Format.imageTiff:
      return _widgetForImage(context, Format.imageTiff, 'TIFF', reader);
    case Format.imageWebP:
      return _widgetForImage(context, Format.imageWebP, 'WebP', reader);
    // regular and file uri may have same mime types on some platforms
    case Format.uri:
    case Format.fileUri:
      // Make sure to request both values before awaiting
      final fileUriFuture = reader.readValue(Format.fileUri);
      final uriFuture = reader.readValue(Format.uri);

      // try file first and if it fails try regular URI
      final fileUri = await fileUriFuture;
      if (fileUri != null) {
        return _RepresentationWidget(
          format: Format.fileUri,
          name: 'File URI',
          synthetized: reader.isSynthetized(Format.fileUri),
          content: Text(fileUri.toString()),
        );
      }
      final uri = await uriFuture;
      if (uri != null) {
        return _RepresentationWidget(
          format: Format.uri,
          name: 'URI',
          synthetized: reader.isSynthetized(Format.uri),
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
