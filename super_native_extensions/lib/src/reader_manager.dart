import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';
import 'reader.dart';

class DataReaderHandle {
  DataReaderHandle._({
    required int handle,
    required FinalizableHandle finalizableHandle,
  })  : _handle = handle,
        _finalizableHandle = finalizableHandle;

  static DataReaderHandle deserialize(dynamic handle) {
    final map = handle as Map;
    return DataReaderHandle._(
      handle: map["handle"],
      finalizableHandle: map["finalizableHandle"],
    );
  }

  final int _handle;
  // ignore: unused_field
  final FinalizableHandle _finalizableHandle;
}

class DataReaderItemHandle {
  DataReaderItemHandle._({
    required int itemHandle,
    required int readerHandle,
  })  : _itemHandle = itemHandle,
        _readerHandle = readerHandle;

  final int _itemHandle;
  final int _readerHandle;
}

class RawReaderManager {
  RawReaderManager._();

  Future<void> dispose(DataReaderHandle reader) async {
    await _channel.invokeMethod("disposeReader", reader._handle);
  }

  Future<List<DataReaderItem>> getItems(DataReaderHandle reader) async {
    final handles =
        await _channel.invokeMethod("getItems", reader._handle) as List<int>;
    return handles
        .map((handle) => DataReaderItem(
            handle: DataReaderItemHandle._(
                itemHandle: handle, readerHandle: reader._handle)))
        .toList(growable: false);
  }

  Future<List<String>> getItemFormats(DataReaderItemHandle handle) async {
    final formats = await _channel.invokeMethod("getItemFormats", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
    }) as List;
    return formats.cast<String>();
  }

  Future<Object?> getItemData(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    return await _channel.invokeMethod("getItemData", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
      "format": format
    });
  }

  final _channel = NativeMethodChannel('DataReaderManager',
      context: superNativeExtensionsContext);

  static final instance = RawReaderManager._();
}
