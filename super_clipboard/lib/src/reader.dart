import 'package:super_data_transfer/super_data_transfer.dart';

import 'common.dart';

class ClipboardReaderItem {
  ClipboardReaderItem._(this.rawItem);

  Future<bool> hasValue(ClipboardKey key) async {
    final platformKey = key.platformKey();
    final allTypes = await rawItem.getAvailableTypes();
    return platformKey
        .readableSystemTypes()
        .any((element) => allTypes.contains(element));
  }

  Future<T?> readValue<T>(ClipboardKey<T> key) async {
    if (!await hasValue(key)) {
      return null;
    }
    final platformKey = key.platformKey();
    for (final type in platformKey.readableSystemTypes()) {
      final value = await rawItem.getDataForType(type);
      if (value != null) {
        final converted = await platformKey.convertFromSystem(value, type);
        if (converted != null) {
          return converted;
        }
      }
    }
    return null;
  }

  final RawClipboardReaderItem rawItem;
}

class ClipboardReader {
  ClipboardReader._(this.rawReader);

  static Future<ClipboardReader> newDefaultReader() async =>
      ClipboardReader._(await RawClipboardReader.newDefaultReader());

  Future<List<ClipboardReaderItem>> getItems() async =>
      (await rawReader.getItems())
          .map((e) => ClipboardReaderItem._(e))
          .toList(growable: false);

  Future<bool> hasValue(ClipboardKey key) async {
    for (final item in await getItems()) {
      if (await item.hasValue(key)) {
        return true;
      }
    }
    return false;
  }

  Future<T?> readValue<T>(ClipboardKey<T> key) async {
    for (final item in await getItems()) {
      final value = await item.readValue(key);
      if (value != null) {
        return value;
      }
    }
    return null;
  }

  final RawClipboardReader rawReader;
}
