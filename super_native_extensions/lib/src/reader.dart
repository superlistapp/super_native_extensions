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
  Future<List<String>> getAvailableFormats() {
    return _mutex.protect(() async {
      _availableFormats ??= await RawReaderManager.instance.getItemFormats(_handle);
      return _availableFormats!;
    });
  }

  Future<Object?> getDataForFormat(String format) {
    return _mutex.protect(() async {
      if (!_dataForFormat.containsKey(format)) {
        _dataForFormat[format] =
            await RawReaderManager.instance.getItemData(_handle, format: format);
      }
      return _dataForFormat[format];
    });
  }

  DataReaderItem({
    required DataReaderItemHandle handle,
  }) : _handle = handle;

  final _mutex = Mutex();

  final DataReaderItemHandle _handle;

  List<String>? _availableFormats;
  final _dataForFormat = <String, Object?>{};
}
