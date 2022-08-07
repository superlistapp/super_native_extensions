import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;
export 'package:super_native_extensions/raw_clipboard.dart'
    show VirtualFileReceiver, Pair;

abstract class DataReader {
  Future<bool> hasValue(EncodableDataFormat f);

  Future<T?> readValue<T>(EncodableDataFormat<T> format);

  Future<VirtualFileReceiver?> getVirtualFileReceiver({
    required DataFormat format,
  });

  /// Web drag&drop only: Will return DataTransferItem for this
  /// reader if available.
  Object? getWebDataTransferItem();

  static DataReader forItem(raw.DataReaderItem item) {
    return _ItemDataReader(item);
  }
}

class _ItemDataReader implements DataReader {
  _ItemDataReader(this.item);

  @override
  Future<bool> hasValue(EncodableDataFormat f) async {
    final formats = await item.getAvailableFormats();
    return formats.any(f.canDecode);
  }

  @override
  Future<T?> readValue<T>(EncodableDataFormat<T> format) async {
    final formats = await item.getAvailableFormats();
    for (final f in formats) {
      if (format.canDecode(f)) {
        final data = await item.getDataForFormat(f).first;
        if (data != null) {
          return format.decode(f, data);
        }
      }
    }
    return null;
  }

  @override
  Future<VirtualFileReceiver?> getVirtualFileReceiver(
      {required DataFormat format}) {
    return item.getVirtualFileReceiver(format: format.primaryFormat);
  }

  @override
  Object? getWebDataTransferItem() {
    return null;
  }

  final raw.DataReaderItem item;
}

class ClipboardReader implements DataReader {
  ClipboardReader._(this.reader);

  static Future<ClipboardReader> readClipboard() async => ClipboardReader._(
      await raw.ClipboardReader.instance.newClipboardReader());

  Future<List<DataReader>> getItems() async => (await reader.getItems())
      .map((e) => _ItemDataReader(e))
      .toList(growable: false);

  @override
  Future<bool> hasValue(EncodableDataFormat format) async {
    for (final item in await getItems()) {
      if (await item.hasValue(format)) {
        return true;
      }
    }
    return false;
  }

  @override
  Future<T?> readValue<T>(EncodableDataFormat<T> format) async {
    for (final item in await getItems()) {
      final value = await item.readValue(format);
      if (value != null) {
        return value;
      }
    }
    return null;
  }

  @override
  Future<VirtualFileReceiver?> getVirtualFileReceiver({
    required Object format,
  }) async {
    return null;
  }

  @override
  Object? getWebDataTransferItem() {
    return null;
  }

  final raw.DataReader reader;
}
