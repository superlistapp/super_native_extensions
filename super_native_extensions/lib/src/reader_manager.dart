import 'reader.dart';

import 'native/reader_manager.dart'
    if (dart.library.js) 'web/reader_manager.dart';

typedef DataReaderHandle = DataReaderHandleImpl;
typedef DataReaderItemHandle = DataReaderItemHandleImpl;

abstract class RawReaderManager {
  static final RawReaderManager instance = RawReaderManagerImpl();

  Future<void> dispose(DataReaderHandle reader);

  Future<List<DataReaderItemHandle>> getItems(DataReaderHandle reader);

  Future<List<String>> getItemFormats(DataReaderItemHandle handle);

  Pair<Future<Object?>, ReadProgress> getItemData(
    DataReaderItemHandle handle, {
    required String format,
  });

  Future<bool> canGetVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
  });

  Pair<Future<String?>, ReadProgress> getVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
    required String targetFolder,
  });
}
