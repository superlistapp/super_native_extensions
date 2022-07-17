import 'mutex.dart';
import 'reader_manager.dart';

class DataReader {
  Future<List<DataReaderItem>> getItems() {
    return _mutex.protect(() async {
      _items ??= await RawReaderManager.instance.getItems(_handle);
      return _items!;
    });
  }

  DataReader({
    required DataReaderHandle handle,
  }) : _handle = handle;

  Future<void> dispose() => RawReaderManager.instance.dispose(_handle);

  final _mutex = Mutex();

  final DataReaderHandle _handle;
  List<DataReaderItem>? _items;
}

class DataReaderItem {
  Future<List<String>> getAvailableTypes() {
    return _mutex.protect(() async {
      _availableTypes ??= await RawReaderManager.instance.getItemTypes(_handle);
      return _availableTypes!;
    });
  }

  Future<Object?> getDataForType(String type) {
    return _mutex.protect(() async {
      if (!_dataForType.containsKey(type)) {
        _dataForType[type] =
            await RawReaderManager.instance.getItemData(_handle, type: type);
      }
      return _dataForType[type];
    });
  }

  DataReaderItem({
    required DataReaderItemHandle handle,
  }) : _handle = handle;

  final _mutex = Mutex();

  final DataReaderItemHandle _handle;

  List<String>? _availableTypes;
  final _dataForType = <String, Object?>{};
}
