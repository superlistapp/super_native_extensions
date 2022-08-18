import 'package:collection/collection.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;
export 'package:super_native_extensions/raw_clipboard.dart'
    show VirtualFileReceiver, Pair, ReadProgress;

abstract class DataReader {
  bool hasValue(DataFormat format) {
    return getFormats([format]).isNotEmpty;
  }

  /// Returns subset of [allFormats] that this reader can provide,
  /// sorted according to priority.
  List<DataFormat> getFormats(List<DataFormat> allFormats);

  /// Attempts to read value for given format. Will return `null` if the value
  /// is not available or the data is virtual (macOS and Windows).
  Future<T?> readValue<T extends Object>(DataFormat<T> format);

  /// Returns whether value for given format is being synthetized. On Windows
  /// DIB images are accessible as PNG (converted on demand), same thing is
  /// done on macOS for TIFF images.
  bool isSynthetized(DataFormat format);

  /// When `true`, data in this format is virtual. It means it might not be
  /// readily available and may need to be retrieved through
  /// [getVirtualFileReceiver] instead of [readValue]. This is the case on macOS
  /// and Windows. On iOS virtual data can be received through both [readValue]
  /// and [getVirtualFileReceiver].
  bool isVirtual(DataFormat format);

  /// Returns suggested file name for the contents (if available).
  Future<String?> getSuggestedName();

  /// Returns virtual file receiver for given format or null if virtual data
  /// for the format is not available. If format not specified returns receiver
  /// for format with highest priority (if any).
  Future<VirtualFileReceiver?> getVirtualFileReceiver({
    VirtualFileFormat? format,
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
    required this.virtualFormats,
  });

  static Future<DataReader> fromItem(raw.DataReaderItem item) async {
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

    return _ItemDataReader._(
      item: item,
      formats: allFormats,
      synthetizedFormats: synthetizedFormats,
      virtualFormats: virtualFormats,
    );
  }

  @override
  List<DataFormat> getFormats(List<DataFormat> allFormats_) {
    final allFormats = List<DataFormat>.of(allFormats_);
    final res = <DataFormat>[];
    for (final f in formats) {
      final decodable = allFormats
          .where((element) => element.canDecode(f))
          .toList(growable: false);
      for (final format in decodable) {
        res.add(format);
        allFormats.remove(format);
      }
    }
    return res;
  }

  @override
  Future<T?> readValue<T extends Object>(DataFormat<T> format) async {
    Future<Object?> provider(PlatformFormat format) async {
      return await item.getDataForFormat(format).first;
    }

    for (final f in formats) {
      if (format.canDecode(f)) {
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
  bool isVirtual(DataFormat<Object> format) {
    return format.receiverFormats.any((f) => virtualFormats.contains(f));
  }

  @override
  Future<String?> getSuggestedName() => item.getSuggestedName();

  @override
  Future<VirtualFileReceiver?> getVirtualFileReceiver({
    VirtualFileFormat? format,
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

/// Clipboard reader exposes contents of the clipboard.
class ClipboardReader extends DataReader {
  ClipboardReader._(this.items);

  /// Individual items of this clipboard reader.
  final List<DataReader> items;

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
    return items.any((item) => item.hasValue(format));
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
    return items.any((item) => item.isSynthetized(format));
  }

  @override
  bool isVirtual(DataFormat<Object> format) {
    return items.any((item) => item.isVirtual(format));
  }

  @override
  Future<VirtualFileReceiver?> getVirtualFileReceiver({
    VirtualFileFormat? format,
  }) async {
    for (final item in items) {
      final receiver = await item.getVirtualFileReceiver(format: format);
      if (receiver != null) {
        return receiver;
      }
    }
    return null;
  }

  @override
  Future<String?> getSuggestedName() async {
    for (final item in items) {
      final name = await item.getSuggestedName();
      if (name != null) {
        return name;
      }
    }
    return null;
  }
}
