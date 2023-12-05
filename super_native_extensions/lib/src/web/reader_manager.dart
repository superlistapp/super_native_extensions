import 'dart:async';

import 'package:flutter/foundation.dart';

import '../reader.dart';
import '../reader_manager.dart';
import 'reader.dart';

class SimpleProgress extends ReadProgress {
  @override
  void cancel() {}

  @override
  ValueListenable<bool> get cancellable => _cancellable;

  @override
  ValueListenable<double?> get fraction => _fraction;

  void done() {
    _fraction.value = 1.0;
  }

  final _cancellable = ValueNotifier(false);
  final _fraction = ValueNotifier<double?>(null);
}

class $DataReaderHandle {
  $DataReaderHandle(this.items);
  final List<$DataReaderItemHandle> items;
}

/// ReaderManagerImpl on web forwards calls to underlying handles.
class ReaderManagerImpl extends ReaderManager {
  @override
  Future<void> dispose(DataReaderHandle reader) async {
    // We don't register the items anywhere so there's nothing to unregister.
  }

  @override
  (Future<Object?>, ReadProgress) getItemData(
    DataReaderItemHandle handle, {
    required String format,
  }) {
    final impl = handle as $DataReaderItemHandle;
    final progress = SimpleProgress();
    final res = impl.getDataForFormat(format);
    final completer = Completer<Object?>();
    res.then((value) {
      progress.done();
      completer.complete(value);
    }).catchError((error) {
      progress.done();
      completer.completeError(error);
    });
    return (completer.future, progress);
  }

  @override
  Future<List<String>> getItemFormats(DataReaderItemHandle handle) {
    final impl = handle as $DataReaderItemHandle;
    return impl.getFormats();
  }

  @override
  Future<String?> getItemSuggestedName(DataReaderItemHandle handle) {
    final impl = handle as $DataReaderItemHandle;
    return impl.suggestedName();
  }

  @override
  Future<bool> itemFormatIsSynthesized(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    return false;
  }

  @override
  Future<List<DataReaderItemHandle>> getItems(DataReaderHandle reader) async {
    final handle = reader as $DataReaderHandle;
    return handle.items.map((e) => e as DataReaderItemHandle).toList();
  }

  @override
  Future<bool> canGetVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
  }) {
    final impl = handle as $DataReaderItemHandle;
    return impl.canGetVirtualFile(format);
  }

  @override
  Future<VirtualFileReceiver?> createVirtualFileReceiver(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    final impl = handle as $DataReaderItemHandle;
    return impl.createVirtualFileReceiver(handle, format: format);
  }

  @override
  Future<String?> formatForFileUri(Uri uri) async {
    return null;
  }

  @override
  VirtualFile createVirtualFileFromUri(Uri uri) {
    throw UnsupportedError('createVirtualFileFromUri is not supported on web');
  }
}
