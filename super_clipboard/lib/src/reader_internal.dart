import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';

import 'format.dart';
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
    required this.virtualFormats,
  });

  static Future<ClipboardDataReader> fromItem(raw.DataReaderItem item) async {
    final allFormats = await item.getAvailableFormats();
    final isSynthetized =
        await Future.wait(allFormats.map((f) => item.isSynthetized(f)));
    final isVirtual =
        await Future.wait(allFormats.map((f) => item.isVirtual(f)));

    final synthetizedFormats = allFormats
        .whereIndexed((index, _) => isSynthetized[index])
        .toList(growable: false);
    final virtualFormats = allFormats
        .whereIndexed((index, _) => isVirtual[index])
        .toList(growable: false);

    return ItemDataReader._(
      item: item,
      formats: allFormats,
      synthetizedFormats: synthetizedFormats,
      virtualFormats: virtualFormats,
    );
  }

  @override
  List<DataFormat> getFormats(List<DataFormat> allFormats) {
    allFormats = List<DataFormat>.of(allFormats);
    final res = <DataFormat>[];
    for (final f in formats) {
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
  ReadProgress? getValue<T extends Object>(
      DataFormat<T> format, ValueChanged<DataReaderValue<T>> onValue) {
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
          onValue(DataReaderValue(value: value));
        }, onError: (e) {
          onValue(DataReaderValue(error: e));
        });
        // Decoder must load value immediately, it can't delay loading across
        // await boundary.
        assert(progress != null,
            'decoder didn\'t request value before async boundary.');
        return progress;
      }
    }

    onValue(DataReaderValue(value: null));
    return null;
  }

  @override
  Future<T?> readValue<T extends Object>(DataFormat<T> format) async {
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
  bool isSynthetized(DataFormat format) {
    return format.decodingFormats.any((f) => synthetizedFormats.contains(f));
  }

  @override
  bool isVirtual(DataFormat format) {
    return format.decodingFormats.any((f) => virtualFormats.contains(f));
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
  final List<PlatformFormat> virtualFormats;
}
