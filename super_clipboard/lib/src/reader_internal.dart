import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'format.dart';
import 'formats_base.dart';
import 'reader_model.dart';
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
    required this.synthesizedFormats,
    required this.virtualReceivers,
    required this.synthesizedFromURIFormat,
  });

  static Future<ClipboardDataReader> fromItem(raw.DataReaderItem item) async {
    final allFormats = await item.getAvailableFormats();
    final isSynthesized =
        await Future.wait(allFormats.map((f) => item.isSynthesized(f)));

    final virtualReceivers = (await Future.wait(
            allFormats.map((f) => item.getVirtualFileReceiver(format: f))))
        .whereNotNull()
        .toList(growable: false);

    final synthesizedFormats = allFormats
        .whereIndexed((index, _) => isSynthesized[index])
        .toList(growable: false);

    String? synthesizedFromURIFormat;

    /// If there are no virtual receivers but there is File URI, we'll
    /// try to synthesize a format from it.
    if (virtualReceivers.isEmpty) {
      for (final format in allFormats) {
        if (Formats.fileUri.canDecode(format)) {
          final uri = await Formats.fileUri.decode(
            format,
            _PlatformDataProvider(
              allFormats,
              (f) => item.getDataForFormat(f).$1,
            ),
          );
          if (uri != null) {
            final format = await raw.DataReader.formatForFileUri(uri);
            if (format != null && !allFormats.contains(format)) {
              synthesizedFromURIFormat = format;
            }
          }
        }
      }
    }

    return ItemDataReader._(
      item: item,
      formats: allFormats,
      synthesizedFormats: synthesizedFormats,
      virtualReceivers: virtualReceivers,
      synthesizedFromURIFormat: synthesizedFromURIFormat,
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
    FileFormat? format,
    AsyncValueChanged<DataReaderFile> onFile, {
    ValueChanged<Object>? onError,
    bool allowVirtualFiles = true,
    bool synthesizeFilesFromURIs = true,
  }) {
    if (format == null &&
        synthesizeFilesFromURIs &&
        synthesizedFromURIFormat != null) {
      format = SimpleFileFormat(fallbackFormats: [synthesizedFromURIFormat!]);
    }

    if (format == null && allowVirtualFiles && virtualReceivers.isNotEmpty) {
      format = SimpleFileFormat(
        fallbackFormats:
            virtualReceivers.map((e) => e.format).toList(growable: false),
      );
    }

    if (format == null && platformFormats.isNotEmpty) {
      format = SimpleFileFormat(fallbackFormats: platformFormats);
    }

    if (format == null) {
      return null;
    }

    final handleError = onError ??
        (error) {
          Zone.current
              .handleUncaughtError(error, AsyncError.defaultStackTrace(error));
        };
    if (synthesizeFilesFromURIs &&
        synthesizedFromURIFormat != null &&
        format.canDecode(synthesizedFromURIFormat!)) {
      return getValue<Uri>(Formats.fileUri, (value) async {
        if (value != null) {
          final file = raw.VirtualFile.fromFileUri(value);
          final adapter = DataReaderVirtualFileAdapter(file);
          final res = onFile(adapter);
          if (res is Future) {
            res.then((_) => adapter.maybeClose());
          }
        } else {
          // This should never happen - the URI was already retrieved once in
          // ItemDataReader.forItem.
          handleError(StateError('Could not retrieve URI'));
        }
      }, onError: (e) {
        handleError(e);
      });
    }

    if (allowVirtualFiles) {
      for (final receiver in virtualReceivers) {
        if (format.canDecode(receiver.format)) {
          final (file, progress) = receiver.receiveVirtualFile();
          file.then(
            (file) async {
              final adapter = DataReaderVirtualFileAdapter(file);
              final res = onFile(adapter);
              if (res is Future) {
                res.then((_) => adapter.maybeClose());
              }
            },
            onError: (e) {
              handleError(e);
            },
          );
          return progress;
        }
      }
    }

    for (final f in formats) {
      if (format.receiverFormats.contains(f)) {
        final (data, progress) = item.getDataForFormat(f);
        data.then((value) {
          final list = value != null ? value as Uint8List : Uint8List(0);
          onFile(DataReaderFileValueAdapter(list));
        }, onError: (e) {
          handleError(e);
        });
        return progress;
      }
    }
    return null;
  }

  @override
  ReadProgress? getValue<T extends Object>(
    ValueFormat<T> format,
    AsyncValueChanged<T?> onValue, {
    ValueChanged<Object>? onError,
  }) {
    final handleError = onError ??
        (error) {
          Zone.current
              .handleUncaughtError(error, AsyncError.defaultStackTrace(error));
        };
    ReadProgress? progress;
    Future<Object?> onGetData(PlatformFormat format) async {
      final (data, itemProgress) = item.getDataForFormat(format);
      progress ??= itemProgress;
      return await data;
    }

    for (final f in formats) {
      if (format.canDecode(f)) {
        final primaryFormat = format.decodingFormats
            .firstWhere((element) => formats.contains(element));
        format
            .decode(primaryFormat, _PlatformDataProvider(formats, onGetData))
            .then((value) {
          onValue(value);
        }, onError: (e) {
          handleError(e);
        });
        // Decoder must load value immediately, it can't delay loading across
        // await boundary.
        assert(progress != null,
            'decoder didn\'t request value before async boundary.');
        return progress;
      }
    }
    return null;
  }

  @override
  Future<T?> readValue<T extends Object>(ValueFormat<T> format) async {
    final c = Completer<T?>();
    final progress = getValue<T>(format, (value) {
      c.complete(value);
    }, onError: (e) {
      c.completeError(e);
    });
    if (progress == null) {
      c.complete(null);
    }
    return c.future;
  }

  @override
  List<PlatformFormat> get platformFormats {
    return [
      ...formats,
      if (synthesizedFromURIFormat != null) synthesizedFromURIFormat!
    ];
  }

  @override
  bool isSynthesized(DataFormat format) {
    return format.decodingFormats.any((f) =>
        synthesizedFormats.contains(f) || //
        synthesizedFromURIFormat == f);
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
  final List<PlatformFormat> synthesizedFormats;
  final List<VirtualFileReceiver> virtualReceivers;
  final PlatformFormat? synthesizedFromURIFormat;
}
