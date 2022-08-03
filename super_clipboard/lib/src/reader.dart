import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'format.dart';

class ClipboardReaderItem {
  ClipboardReaderItem._(this.item);

  Future<bool> hasValue(EncodableDataFormat f) async {
    final formats = await item.getAvailableFormats();
    return formats.any(f.canDecode);
  }

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

  final raw.DataReaderItem item;
}

class ClipboardReader {
  ClipboardReader._(this.reader);

  static Future<ClipboardReader> readClipboard() async => ClipboardReader._(
      await raw.ClipboardReader.instance.newClipboardReader());

  Future<List<ClipboardReaderItem>> getItems() async =>
      (await reader.getItems())
          .map((e) => ClipboardReaderItem._(e))
          .toList(growable: false);

  Future<bool> hasValue(EncodableDataFormat format) async {
    for (final item in await getItems()) {
      if (await item.hasValue(format)) {
        return true;
      }
    }
    return false;
  }

  Future<T?> readValue<T>(EncodableDataFormat<T> format) async {
    for (final item in await getItems()) {
      final value = await item.readValue(format);
      if (value != null) {
        return value;
      }
    }
    return null;
  }

  final raw.DataReader reader;
}
