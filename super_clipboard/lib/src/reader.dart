import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_native_extensions/raw_clipboard.dart';

class ClipboardReaderItem {
  ClipboardReaderItem._(this.rawItem);

  Future<bool> hasValue(ClipboardType key) async {
    final platformKey = key.platformType();
    final allTypes = await rawItem.getAvailableTypes();
    return platformKey
        .readableSystemTypes()
        .any((element) => allTypes.contains(element));
  }

  Future<T?> readValue<T>(ClipboardType<T> key) async {
    if (!await hasValue(key)) {
      return null;
    }
    final platformKey = key.platformType();
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

  final DataReaderItem rawItem;
}

class ClipboardReader {
  ClipboardReader._(this.rawReader);

  static Future<ClipboardReader> newDefaultReader() async =>
      ClipboardReader._(await RawClipboardReader.instance.newClipboardReader());

  Future<List<ClipboardReaderItem>> getItems() async =>
      (await rawReader.getItems())
          .map((e) => ClipboardReaderItem._(e))
          .toList(growable: false);

  Future<bool> hasValue(ClipboardType key) async {
    for (final item in await getItems()) {
      if (await item.hasValue(key)) {
        return true;
      }
    }
    return false;
  }

  Future<T?> readValue<T>(ClipboardType<T> key) async {
    for (final item in await getItems()) {
      final value = await item.readValue(key);
      if (value != null) {
        return value;
      }
    }
    return null;
  }

  final DataReader rawReader;
}
