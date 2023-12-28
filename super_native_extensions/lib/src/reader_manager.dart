import 'reader.dart';

import 'native/reader_manager.dart'
    if (dart.library.js) 'web/reader_manager.dart';

// There is a separate $DataReaderHandle and $DataReaderItemHandle definition for
// web and native. The typedef with $prefix is used within the web/native section
// so that dart analyzer can find the correct type.
typedef DataReaderHandle = $DataReaderHandle;
typedef DataReaderItemHandle = $DataReaderItemHandle;

abstract class ReaderManager {
  static final ReaderManager instance = ReaderManagerImpl();

  Future<void> dispose(DataReaderHandle reader);

  Future<List<DataReaderItemHandle>> getItems(DataReaderHandle reader);

  Future<List<String>> getItemFormats(DataReaderItemHandle handle);

  (Future<Object?>, ReadProgress) getItemData(
    DataReaderItemHandle handle, {
    required String format,
  });

  Future<List<DataReaderItemInfo>> getItemInfo(
    Iterable<DataReaderItemHandle> handles, {
    Duration? timeout,
  });

  Future<String?> formatForFileUri(Uri uri);

  VirtualFile createVirtualFileFromUri(Uri uri);
}
