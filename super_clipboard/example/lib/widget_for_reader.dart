import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_clipboard_example/main.dart';

/// Builds widget containing information for data reader.
Future<Widget> buildWidgetForReader(DataReader reader, int index) async {
  final itemFormats = reader.getFormats([
    ...Format.standardFormats,
    formatCustom,
  ]);

  // Request all data before awaiting
  final futures = itemFormats.map((e) => _widgetForFormat(e, reader));

  // Now await all futures
  final widgets = await Future.wait(futures);
  final children =
      widgets.where((element) => element != null).cast<Widget>().toList();

  List<String>? nativeFormats = await reader.rawReader?.getAvailableFormats();

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

  final String name;
  final bool synthetized;
  final Widget content;
}

Future<Widget?> _widgetForImage(
    DataFormat<Uint8List> format, String name, DataReader reader) async {
  final image = await reader.readValue(format);
  if (image == null) {
    return null;
  } else {
    return _RepresentationWidget(
      name: 'Image ($name)',
      synthetized: reader.isSynthetized(format),
      content: Container(
        padding: const EdgeInsets.only(top: 4),
        alignment: Alignment.centerLeft,
        child: Image.memory(image),
      ),
    );
  }
}

Future<Widget?> _widgetForFormat(DataFormat format, DataReader reader) async {
  switch (format) {
    case Format.plainText:
      final text = await reader.readValue(Format.plainText);
      if (text == null) {
        return null;
      } else {
        // Sometimes macOS uses CR for line break;
        final sanitized = text.replaceAll(RegExp('\r[\n]?'), '\n');
        return _RepresentationWidget(
          name: 'Plain Text',
          synthetized: reader.isSynthetized(Format.plainText),
          content: Text(sanitized),
        );
      }
    case Format.html:
      final html = await reader.readValue(Format.html);
      if (html == null) {
        return null;
      } else {
        return _RepresentationWidget(
          name: 'HTML Text',
          synthetized: reader.isSynthetized(Format.html),
          content: Text(html),
        );
      }
    case Format.imagePng:
      return _widgetForImage(Format.imagePng, 'PNG', reader);
    case Format.imageJpeg:
      return _widgetForImage(Format.imageJpeg, 'JPEG', reader);
    case Format.imageGif:
      return _widgetForImage(Format.imageGif, 'GIF', reader);
    case Format.imageTiff:
      return _widgetForImage(Format.imageTiff, 'TIFF', reader);
    case Format.imageWebP:
      return _widgetForImage(Format.imageWebP, 'WebP', reader);
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
          name: 'File URI',
          synthetized: reader.isSynthetized(Format.fileUri),
          content: Text(fileUri.toString()),
        );
      }
      final uri = await uriFuture;
      if (uri != null) {
        return _RepresentationWidget(
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
