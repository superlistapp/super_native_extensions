import 'package:flutter/foundation.dart';
import 'package:super_native_extensions/src/reader.dart';
import 'package:super_native_extensions/src/reader_manager.dart';

class RawReaderManagerImpl extends RawReaderManager {
  @override
  Future<void> dispose(DataReaderHandle reader) {
    // TODO: implement dispose
    throw UnimplementedError();
  }

  @override
  ReadProgress getItemData(DataReaderItemHandle handle,
      {required String format, required ValueChanged<GetDataResult> onData}) {
    // TODO: implement getItemData
    throw UnimplementedError();
  }

  @override
  Future<List<String>> getItemFormats(DataReaderItemHandle handle) {
    // TODO: implement getItemFormats
    throw UnimplementedError();
  }

  @override
  Future<List<DataReaderItem>> getItems(DataReaderHandle reader) {
    // TODO: implement getItems
    throw UnimplementedError();
  }

  @override
  Future<bool> canGetVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    return false;
  }

  @override
  ReadProgress getVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
    required String targetFolder,
    required ValueChanged<DataResult<String?>> onResult,
  }) {
    throw UnsupportedError('Virtual files are not supported on web');
  }
}
