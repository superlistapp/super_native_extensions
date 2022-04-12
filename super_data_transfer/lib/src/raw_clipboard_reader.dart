import 'package:nativeshell_core/nativeshell_core.dart';

import 'mutex.dart';
import 'context.dart';

class RawClipboardReader {
  Future<List<RawClipboardReaderItem>> getItems() {
    return _mutex.protect(() async {
      _items ??= await _RawClipboardReaderManager.instance.getItems(this);
      return _items!;
    });
  }

  RawClipboardReader._({
    required int handle,
    required FinalizableHandle finalizableHandle,
  })  : _handle = handle,
        _finalizableHandle = finalizableHandle;

  /// Returns clipboard reader for current clipboard. Note that on some platforms
  /// the clipboard content for single reader will not change during the lifetime
  /// of the reader. Also the content is cached lazily. If you need updated information
  /// create a new reader.
  static Future<RawClipboardReader> newDefaultReader() =>
      _RawClipboardReaderManager.instance.defaultReader();

  Future<void> dispose() => _RawClipboardReaderManager.instance.dispose(this);

  final _mutex = Mutex();

  final int _handle;
  // ignore: unused_field
  final FinalizableHandle _finalizableHandle;
  List<RawClipboardReaderItem>? _items;
}

class RawClipboardReaderItem {
  Future<List<String>> getAvailableTypes() {
    return _mutex.protect(() async {
      _availableTypes ??=
          await _RawClipboardReaderManager.instance.getItemTypes(this);
      return _availableTypes!;
    });
  }

  Future<Object?> getDataForType(String type) {
    return _mutex.protect(() async {
      if (!_dataForType.containsKey(type)) {
        _dataForType[type] = await _RawClipboardReaderManager.instance
            .getItemData(this, type: type);
      }
      return _dataForType[type];
    });
  }

  RawClipboardReaderItem._({
    required int itemHandle,
    required int readerHandle,
  })  : _itemHandle = itemHandle,
        _readerHandle = readerHandle;

  final _mutex = Mutex();

  final int _itemHandle;
  final int _readerHandle;

  List<String>? _availableTypes;
  final _dataForType = <String, Object?>{};
}

//
//
//

class _RawClipboardReaderManager {
  _RawClipboardReaderManager._();

  Future<RawClipboardReader> defaultReader() async {
    final res = await _channel.invokeMethod("newDefaultReader") as Map;
    return RawClipboardReader._(
      handle: res["handle"],
      finalizableHandle: res["finalizableHandle"],
    );
  }

  Future<void> dispose(RawClipboardReader reader) async {
    await _channel.invokeMethod("disposeReader", reader._handle);
  }

  Future<List<RawClipboardReaderItem>> getItems(
      RawClipboardReader reader) async {
    final handles =
        await _channel.invokeMethod("getItems", reader._handle) as List<int>;
    return handles
        .map((handle) => RawClipboardReaderItem._(
            itemHandle: handle, readerHandle: reader._handle))
        .toList(growable: false);
  }

  Future<List<String>> getItemTypes(RawClipboardReaderItem item) async {
    final types = await _channel.invokeMethod("getItemTypes", {
      "itemHandle": item._itemHandle,
      "readerHandle": item._readerHandle,
    }) as List;
    return types.cast<String>();
  }

  Future<Object?> getItemData(
    RawClipboardReaderItem item, {
    required String type,
  }) async {
    return await _channel.invokeMethod("getItemData", {
      "itemHandle": item._itemHandle,
      "readerHandle": item._readerHandle,
      "dataType": type
    });
  }

  final _channel = NativeMethodChannel('ClipboardReaderManager',
      context: superDataTransferContext);

  static final instance = _RawClipboardReaderManager._();
}
