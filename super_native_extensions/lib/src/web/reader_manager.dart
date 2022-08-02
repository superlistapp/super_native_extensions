import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:super_native_extensions/src/reader.dart';
import 'package:super_native_extensions/src/reader_manager.dart';

class SimpleProgress extends ReadProgress {
  @override
  void cancel() {}

  @override
  ValueListenable<bool> get cancellable => _cancellable;

  @override
  ValueListenable<double?> get fraction => _fraction;

  final _cancellable = ValueNotifier(false);
  final _fraction = ValueNotifier<double?>(null);
}

class DataReaderHandleImpl {
  DataReaderHandleImpl(this.items);
  final List<DataReaderItemHandleImpl> items;
}

abstract class DataReaderItemHandleImpl {
  Future<List<String>> getFormats();
  Future<Object?> getDataForFormat(String format);
}

class RawReaderManagerImpl extends RawReaderManager {
  @override
  Future<void> dispose(DataReaderHandle reader) async {
    // we don't register the items anywhere so there's nothing to undergister
  }

  @override
  Pair<Future<Object?>, ReadProgress> getItemData(
    DataReaderItemHandle handle, {
    required String format,
  }) {
    final impl = handle as DataReaderItemHandleImpl;
    final progress = SimpleProgress();
    final res = impl.getDataForFormat(format);
    final completer = Completer<Object?>();
    res.then((value) {
      progress._fraction.value = 1.0;
      completer.complete(value);
    }).catchError((error) {
      progress._fraction.value = 1.0;
      completer.completeError(error);
    });
    return Pair(completer.future, progress);
  }

  @override
  Future<List<String>> getItemFormats(DataReaderItemHandle handle) {
    final impl = handle as DataReaderItemHandleImpl;
    return impl.getFormats();
  }

  @override
  Future<List<DataReaderItemHandle>> getItems(DataReaderHandle reader) async {
    final handle = reader as DataReaderHandleImpl;
    return handle.items.map((e) => e as DataReaderItemHandle).toList();
  }

  @override
  Future<bool> canGetVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    return false;
  }

  @override
  Pair<Future<String?>, ReadProgress> getVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
    required String targetFolder,
  }) {
    throw UnsupportedError('Virtual files are not supported on web');
  }
}
