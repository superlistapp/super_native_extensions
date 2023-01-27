import 'dart:async';
import 'dart:typed_data';

import 'package:collection/collection.dart';

import 'format.dart';
import 'reader_value.dart';
import 'standard_formats.dart';
import 'reader.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

extension FormatExtension<T extends Object> on DataFormat<T> {
  List<String> get decodingFormats {
    if (this is ValueFormat) {
      return (this as ValueFormat).codec.decodingFormats;
    } else if (this is FileFormat) {
      return (this as FileFormat).receiverFormats;
    } else {
      throw StateError('Unknown format type');
    }
  }

  bool canDecode(PlatformFormat format) {
    return decodingFormats.contains(format);
  }

  Future<T?> decode(
      PlatformFormat format, PlatformDataProvider provider) async {
    if (this is ValueFormat) {
      return (this as ValueFormat<T>).codec.decode(provider, format);
    } else if (this is FileFormat) {
      return (await provider.getData(format)) as T?;
    } else {
      throw StateError('Unknown format type');
    }
  }
}

class _PlatformDataProvider extends PlatformDataProvider {
  _PlatformDataProvider(this.formats, this.onGetData);

  final List<PlatformFormat> formats;
  final Future<Object?> Function(PlatformFormat) onGetData;

  @override
  List<PlatformFormat> getAllFormats() => formats;

  @override
  Future<Object?> getData(PlatformFormat format) => onGetData(format);
}

class ItemDataReader extends ClipboardDataReader {
  ItemDataReader._({
    required this.item,
    required this.formats,
    required this.synthetizedFormats,
    required this.virtualReceivers,
    required this.synthetizedFromURIFormat,
  });

  static Future<ClipboardDataReader> fromItem(raw.DataReaderItem item) async {
    final allFormats = await item.getAvailableFormats();
    final isSynthetized =
        await Future.wait(allFormats.map((f) => item.isSynthetized(f)));

    final virtualReceivers = (await Future.wait(
            allFormats.map((f) => item.getVirtualFileReceiver(format: f))))
        .whereNotNull()
        .toList(growable: false);

    final synthetizedFormats = allFormats
        .whereIndexed((index, _) => isSynthetized[index])
        .toList(growable: false);

    String? synthetizedFromURIFormat;

    /// If there are no virtual receivers but there is File URI, we'll
    /// try to synthetize a format from it.
    if (virtualReceivers.isEmpty) {
      for (final format in allFormats) {
        if (Formats.fileUri.canDecode(format)) {
          final uri = await Formats.fileUri.decode(
            format,
            _PlatformDataProvider(
              allFormats,
              (f) => item.getDataForFormat(f).first,
            ),
          );
          if (uri != null) {
            final format = await raw.DataReader.formatForFileUri(uri);
            if (format != null && !allFormats.contains(format)) {
              synthetizedFromURIFormat = format;
            }
          }
        }
      }
    }

    return ItemDataReader._(
      item: item,
      formats: allFormats,
      synthetizedFormats: synthetizedFormats,
      virtualReceivers: virtualReceivers,
      synthetizedFromURIFormat: synthetizedFromURIFormat,
    );
  }

  @override
  List<DataFormat> getFormats(List<DataFormat> allFormats) {
    allFormats = List.of(allFormats);
    final res = <DataFormat>[];
    for (final f in platformFormats) {
      final decodable = allFormats
          .where((element) => element.canDecode(f))
          .toList(growable: false)
        // sort decoders that can handle this format by how
        // far it is in their supported format lists
        ..sort(
          (a, b) => a.decodingFormats
              .indexOf(f)
              .compareTo(b.decodingFormats.indexOf(f)),
        );
      for (final format in decodable) {
        res.add(format);
        allFormats.remove(format);
      }
    }
    return res;
  }

  @override
  ReadProgress? getFile(
    FileFormat format,
    AsyncValueChanged<DataReaderResult<DataReaderFile>> onFile, {
    bool allowVirtualFiles = true,
    bool synthetizeFilesFromURIs = true,
  }) {
    if (synthetizeFilesFromURIs &&
        synthetizedFromURIFormat != null &&
        format.canDecode(synthetizedFromURIFormat!)) {
      return getValue<Uri>(Formats.fileUri, (value) async {
        if (value.value != null) {
          final file = raw.VirtualFile.fromFileUri(value.value!);
          final adapter = DataReaderVirtualFileAdapter(file);
          final res = onFile(DataReaderResult(value: adapter));
          if (res is Future) {
            res.then((_) => adapter.maybeDispose());
          }
        } else {
          onFile(DataReaderResult(error: value.error));
        }
      });
    }

    if (allowVirtualFiles) {
      for (final receiver in virtualReceivers) {
        if (format.canDecode(receiver.format)) {
          final file = receiver.receiveVirtualFile();
          file.first.then(
            (file) async {
              final adapter = DataReaderVirtualFileAdapter(file);
              final res = onFile(DataReaderResult(value: adapter));
              if (res is Future) {
                res.then((_) => adapter.maybeDispose());
              }
            },
            onError: (e) {
              onFile(DataReaderResult(error: e));
            },
          );
          return file.second;
        }
      }
    }

    for (final f in formats) {
      if (format.receiverFormats.contains(f)) {
        final data = item.getDataForFormat(f);
        data.first.then((value) {
          if (value != null) {
            final list = value as Uint8List;
            onFile(DataReaderResult(value: DataReaderFileValueAdapter(list)));
          } else {
            onFile(DataReaderResult());
          }
        }, onError: (e) {
          onFile(DataReaderResult(error: e));
        });
        return data.second;
      }
    }
    onFile(DataReaderResult());
    return null;
  }

  @override
  ReadProgress? getValue<T extends Object>(
    ValueFormat<T> format,
    AsyncValueChanged<DataReaderResult<T>> onValue,
  ) {
    ReadProgress? progress;
    Future<Object?> onGetData(PlatformFormat format) async {
      final data = item.getDataForFormat(format);
      progress ??= data.second;
      return await data.first;
    }

    for (final f in formats) {
      if (format.canDecode(f)) {
        final primaryFormat = format.decodingFormats
            .firstWhere((element) => formats.contains(element));
        format
            .decode(primaryFormat, _PlatformDataProvider(formats, onGetData))
            .then((value) {
          onValue(DataReaderResult(value: value));
        }, onError: (e) {
          onValue(DataReaderResult(error: e));
        });
        // Decoder must load value immediately, it can't delay loading across
        // await boundary.
        assert(progress != null,
            'decoder didn\'t request value before async boundary.');
        return progress;
      }
    }
    onValue(DataReaderResult());
    return null;
  }

  @override
  Future<T?> readValue<T extends Object>(ValueFormat<T> format) async {
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

  @override
  List<PlatformFormat> get platformFormats {
    return [
      ...formats,
      if (synthetizedFromURIFormat != null) synthetizedFromURIFormat!
    ];
  }

  @override
  bool isSynthetized(DataFormat format) {
    return format.decodingFormats.any((f) =>
        synthetizedFormats.contains(f) || //
        synthetizedFromURIFormat == f);
  }

  @override
  bool isVirtual(DataFormat format) {
    return format.decodingFormats
        .any((f) => virtualReceivers.any((rec) => rec.format == f));
  }

  @override
  Future<String?> getSuggestedName() => item.getSuggestedName();

  @override
  Future<VirtualFileReceiver?> getVirtualFileReceiver({
    FileFormat? format,
  }) async {
    final formats = format?.receiverFormats ?? await item.getAvailableFormats();
    for (final format in formats) {
      final receiver = await item.getVirtualFileReceiver(format: format);
      if (receiver != null) {
        return receiver;
      }
    }
    return null;
  }

  @override
  raw.DataReaderItem? get rawReader => item;

  final raw.DataReaderItem item;
  final List<PlatformFormat> formats;
  final List<PlatformFormat> synthetizedFormats;
  final List<VirtualFileReceiver> virtualReceivers;
  final PlatformFormat? synthetizedFromURIFormat;
}
