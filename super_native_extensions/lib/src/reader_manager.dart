import 'package:flutter/foundation.dart';

import 'reader.dart';

import 'native/reader_manager.dart'
    if (dart.library.js) 'web/reader_manager.dart';

typedef DataReaderHandle = DataReaderHandleImpl;
typedef DataReaderItemHandle = DataReaderItemHandleImpl;

abstract class RawReaderManager {
  static final RawReaderManager instance = RawReaderManagerImpl();

  Future<void> dispose(DataReaderHandle reader);

  Future<List<DataReaderItem>> getItems(DataReaderHandle reader);

  Future<List<String>> getItemFormats(DataReaderItemHandle handle);

  ReadProgress getItemData(
    DataReaderItemHandle handle, {
    required String format,
    required ValueChanged<GetDataResult> onData,
  });

  Future<bool> canGetVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
  });

  ReadProgress getVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
    required String targetFolder,
    required ValueChanged<DataResult<String?>> onResult,
  });
}
