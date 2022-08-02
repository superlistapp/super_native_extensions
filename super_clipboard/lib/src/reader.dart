import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'format.dart';

class ClipboardReaderItem {
  ClipboardReaderItem._(this.item);

  Future<bool> hasValue(DataFormat f) async {
    final formats = await item.getAvailableFormats();
    return formats.any(f.canHandle);
  }

  Future<T?> readValue<T>(DataFormat<T> key) async {
    final formats = await item.getAvailableFormats();
    for (final format in formats) {
      if (key.canHandle(format)) {
        final data = await item.getDataForFormat(format).first;
        if (data != null) {
          return key.decode(format, data);
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
      await raw.RawClipboardReader.instance.newClipboardReader());

  Future<List<ClipboardReaderItem>> getItems() async =>
      (await reader.getItems())
          .map((e) => ClipboardReaderItem._(e))
          .toList(growable: false);

  Future<bool> hasValue(DataFormat format) async {
    for (final item in await getItems()) {
      if (await item.hasValue(format)) {
        return true;
      }
    }
    return false;
  }

  Future<T?> readValue<T>(DataFormat<T> format) async {
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
