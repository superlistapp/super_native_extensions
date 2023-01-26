import 'reader.dart';

import 'native/reader_manager.dart'
    if (dart.library.js) 'web/reader_manager.dart';

typedef DataReaderHandle = DataReaderHandleImpl;
typedef DataReaderItemHandle = DataReaderItemHandleImpl;

abstract class ReaderManager {
  static final ReaderManager instance = ReaderManagerImpl();

  Future<void> dispose(DataReaderHandle reader);

  Future<List<DataReaderItemHandle>> getItems(DataReaderHandle reader);

  Future<List<String>> getItemFormats(DataReaderItemHandle handle);

  Pair<Future<Object?>, ReadProgress> getItemData(
    DataReaderItemHandle handle, {
    required String format,
  });

  Future<bool> itemFormatIsSynthetized(
    DataReaderItemHandle handle, {
    required String format,
  });

  Future<String?> getItemSuggestedName(DataReaderItemHandle handle);

  Future<bool> canGetVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
  });

  Future<VirtualFileReceiver?> createVirtualFileReceiver(
    DataReaderItemHandle handle, {
    required String format,
  });

  Future<String?> formatForFileUri(Uri uri);
}
