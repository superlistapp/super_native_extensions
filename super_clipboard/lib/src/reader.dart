import 'package:collection/collection.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;
export 'package:super_native_extensions/raw_clipboard.dart'
    show VirtualFileReceiver, Pair;

abstract class DataReader {
  bool hasValue(DataFormat format) {
    return getFormats([format]).isNotEmpty;
  }

  /// Returns subset of [allFormats] that this reader can provide,
  /// sorted according to priority.
  List<DataFormat> getFormats(List<DataFormat> allFormats);

  /// Attempts to read value for given format.
  Future<T?> readValue<T extends Object>(DataFormat<T> format);

  /// Returns whether value for given format is being synthetized. On Windows
  /// DIB images are accessible as PNG (converted on demand), same thing is
  /// done on macOS for TIFF images.
  bool isSynthetized(DataFormat format);

  /// Returns suggested file name for the contents (if available).
  String? get suggestedName;

  Future<VirtualFileReceiver?> getVirtualFileReceiver({
    required VirtualFileFormat format,
  });

  /// If this reader is backed by raw DataReaderItem returns it.
  raw.DataReaderItem? get rawReader => null;

  static Future<DataReader> forItem(raw.DataReaderItem item) async =>
      _ItemDataReader.fromItem(item);
}

class _ItemDataReader extends DataReader {
  _ItemDataReader._({
    required this.item,
    required this.formats,
    required this.synthetizedFormats,
    required this.suggestedName,
  });

  static Future<DataReader> fromItem(raw.DataReaderItem item) async {
    final allFormats = await item.getAvailableFormats();
    final isSynthetized =
        await Future.wait(allFormats.map((f) => item.isSynthetized(f)));

    final synthetizedFormats = allFormats
        .whereIndexed((index, _) => isSynthetized[index])
        .toList(growable: false);

    return _ItemDataReader._(
      item: item,
      formats: allFormats,
      synthetizedFormats: synthetizedFormats,
      suggestedName: await item.getSuggestedName(),
    );
  }

  @override
  List<DataFormat> getFormats(List<DataFormat> allFormats_) {
    final allFormats = List<DataFormat>.of(allFormats_);
    final res = <DataFormat>[];
    for (final f in formats) {
      final format =
          allFormats.firstWhereOrNull((element) => element.canDecode(f));
      if (format != null) {
        res.add(format);
        allFormats.remove(format);
      }
    }
    return res;
  }

  @override
  Future<T?> readValue<T extends Object>(DataFormat<T> format) async {
    for (final f in formats) {
      if (format.canDecode(f)) {
        Future<Object?> provider(PlatformFormat format) async {
          return await item.getDataForFormat(format).first;
        }

        return format.decode(f, provider);
      }
    }
    return null;
  }

  @override
  bool isSynthetized(DataFormat format) {
    return format.receiverFormats.any((f) => synthetizedFormats.contains(f));
  }

  @override
  final String? suggestedName;

  @override
  Future<VirtualFileReceiver?> getVirtualFileReceiver(
      {required VirtualFileFormat format}) async {
    for (final format in format.receiverFormats) {
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
}

class ClipboardReader extends DataReader {
  ClipboardReader._(this.items);

  static Future<ClipboardReader> readClipboard() async {
    final reader = await raw.ClipboardReader.instance.newClipboardReader();
    final readerItems = await reader.getItems();
    final items = <DataReader>[];
    for (final item in readerItems) {
      items.add(await DataReader.forItem(item));
    }
    return ClipboardReader._(items);
  }

  @override
  List<DataFormat> getFormats(List<DataFormat> allFormats) {
    final res = <DataFormat>[];
    for (final item in items) {
      final itemFormats = item.getFormats(allFormats);
      for (final format in itemFormats) {
        if (!res.contains(format)) {
          res.add(format);
        }
      }
    }
    return res;
  }

  @override
  bool hasValue(DataFormat format) {
    for (final item in items) {
      if (item.hasValue(format)) {
        return true;
      }
    }
    return false;
  }

  @override
  Future<T?> readValue<T extends Object>(DataFormat<T> format) async {
    for (final item in items) {
      final value = await item.readValue(format);
      if (value != null) {
        return value;
      }
    }
    return null;
  }

  @override
  bool isSynthetized(DataFormat<Object> format) {
    for (final item in items) {
      if (item.isSynthetized(format)) {
        return true;
      }
    }
    return false;
  }

  @override
  Future<VirtualFileReceiver?> getVirtualFileReceiver({
    required Object format,
  }) async {
    return null;
  }

  @override
  String? get suggestedName => null;

  final List<DataReader> items;
}
